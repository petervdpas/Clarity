//! The felt table with its dice as Bevy draws it, headless: the camera renders
//! into a picture of its own, and each update reads that picture back, a frame
//! or two later, as plain RGBA pixels for the part to put in its place.
//!
//! Bevy has no window and no window loop here: `step` runs one `app.update()`.

use std::f32::consts::PI;

use bevy::app::PluginsState;
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::window::ExitCondition;

/// The most dice the table holds.
pub const MOST: usize = 6;
/// How long a throw takes, in seconds, before the first die lands.
const THROW: f32 = 1.4;
/// How much later each next die lands.
const STAGGER: f32 = 0.12;
/// How far apart the dice rest.
const SPACING: f32 = 1.7;
/// How many updates run after anything changed, so the picture read back shows it.
const SETTLE: u32 = 8;
/// How many updates run at the start: Bevy makes its shaders meanwhile, and
/// the dice show only once those are ready.
const WARM_UP: u32 = 120;
/// The camera's distance from the table's middle, nearest and farthest.
const NEAREST: f32 = 6.0;
const FARTHEST: f32 = 15.0;

/// The table, rendering into a picture `width` by `height` pixels.
pub struct Scene {
    app: App,
    width: u32,
    height: u32,
    /// Updates still to run before the picture is known to show the table as it is.
    settle: u32,
}

/// What one update brought.
pub struct Step {
    /// The dice all landed: their total and faces.
    pub landed: Option<(i64, String)>,
    /// A new picture was read back.
    pub fresh: bool,
    /// The table still moves, or its picture is not yet read back: update again soon.
    pub busy: bool,
}

/// The last picture read back: RGBA rows, each `row_bytes` long (wgpu pads them to 256 bytes).
#[derive(Resource, Default)]
struct Shown {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    row_bytes: usize,
    fresh: bool,
}

/// The picture the camera renders into, at its size.
#[derive(Resource)]
struct Target {
    image: Handle<Image>,
    width: u32,
    height: u32,
}

/// How many dice are on the table.
#[derive(Resource)]
struct Count(usize);

/// A roll was asked for.
#[derive(Resource, Default)]
struct RollAsked(bool);

/// What the last throw showed, once every die has landed, for the part to tell.
#[derive(Resource, Default)]
struct Landed(Option<(i64, String)>);

/// Whether the dice are in the air, and for how long.
#[derive(Resource, Default)]
struct Throw {
    rolling: bool,
    elapsed: f32,
}

/// Where the camera looks from: turned `yaw` about the table's up, `distance` away.
#[derive(Resource)]
struct View {
    yaw: f32,
    distance: f32,
}

/// The camera, which follows the view.
#[derive(Component)]
struct Eye;

/// A small random number source, seeded from the clock.
#[derive(Resource)]
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        // xorshift64
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A number from 0 up to 1.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A face, 1 to 6.
    fn face(&mut self) -> u8 {
        (self.next() % 6) as u8 + 1
    }
}

/// One die: what it shows and how it moves while thrown.
#[derive(Component)]
struct Die {
    index: usize,
    face: u8,
    from: Vec3,
    to: Vec3,
    rest: Quat,
    spin_axis: Vec3,
    spin: f32,
}

/// A picture the camera renders into and the readback copies from.
fn target_image(width: u32, height: u32) -> Image {
    let mut image = Image::new_target_texture(width, height, TextureFormat::Rgba8UnormSrgb, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image
}

/// The bytes of one row as the readback gives them: rounded up to 256.
fn row_bytes(width: u32) -> usize {
    (width as usize * 4).div_ceil(256) * 256
}

impl Scene {
    /// Starts Bevy headless with the table, rendering `width` by `height` pixels.
    /// Fails, saying why, when no renderer could be had.
    pub fn new(width: u32, height: u32) -> Result<Scene, String> {
        let (width, height) = (width.max(1), height.max(1));
        let made = std::panic::catch_unwind(move || Scene::start(width, height));
        match made {
            Ok(scene) => Ok(scene),
            Err(panic) => Err(panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()),
        }
    }

    fn start(width: u32, height: u32) -> Scene {
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .build()
                // The program has its own Ctrl+C.
                .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
                .set(bevy::window::WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    close_when_requested: false,
                    ..default()
                }),
        );
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1) | 1;
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(target_image(width, height));
        app.insert_resource(ClearColor(Color::srgb(0.05, 0.16, 0.10)))
            .insert_resource(Count(2))
            .insert_resource(Random(seed))
            .insert_resource(View { yaw: 0.0, distance: 10.7 })
            .insert_resource(Target { image, width, height })
            .init_resource::<Shown>()
            .init_resource::<RollAsked>()
            .init_resource::<Landed>()
            .init_resource::<Throw>()
            .add_systems(Startup, set_table)
            .add_systems(Update, (start_throw, throw, lay_out, look).chain());

        // As Bevy's own runner does before its first update.
        while app.plugins_state() == PluginsState::Adding {
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
        app.finish();
        app.cleanup();
        Scene { app, width, height, settle: WARM_UP }
    }

    /// The picture's size in pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Renders at a new size from the next update on.
    pub fn resize(&mut self, width: u32, height: u32) {
        let (width, height) = (width.max(1), height.max(1));
        if (width, height) == (self.width, self.height) {
            return;
        }
        self.width = width;
        self.height = height;
        let world = self.app.world_mut();
        let image = world.resource_mut::<Assets<Image>>().add(target_image(width, height));
        *world.resource_mut::<Target>() = Target { image: image.clone(), width, height };
        let mut cameras = world.query_filtered::<&mut RenderTarget, With<Eye>>();
        for mut target in cameras.iter_mut(world) {
            *target = RenderTarget::from(image.clone());
        }
        let mut readbacks = world.query::<&mut Readback>();
        for mut readback in readbacks.iter_mut(world) {
            *readback = Readback::texture(image.clone());
        }
        self.settle = SETTLE;
    }

    /// Shows `count` dice, 1 to 6.
    pub fn set_count(&mut self, count: i64) {
        let count = count.clamp(1, MOST as i64) as usize;
        let mut held = self.app.world_mut().resource_mut::<Count>();
        if held.0 != count {
            held.0 = count;
            self.settle = SETTLE;
        }
    }

    /// Throws the dice.
    pub fn roll(&mut self) {
        self.app.world_mut().resource_mut::<RollAsked>().0 = true;
        self.settle = SETTLE;
    }

    /// Whether it has updates to run: the table moves, or its picture is not yet read back.
    pub fn busy(&self) -> bool {
        self.settle > 0
    }

    /// Turns the view about the table's up by `by` radians.
    pub fn turn(&mut self, by: f32) {
        self.app.world_mut().resource_mut::<View>().yaw += by;
        self.settle = SETTLE;
    }

    /// Comes nearer (below 1) or goes farther (above 1).
    pub fn zoom(&mut self, by: f32) {
        let mut view = self.app.world_mut().resource_mut::<View>();
        view.distance = (view.distance * by).clamp(NEAREST, FARTHEST);
        self.settle = SETTLE;
    }

    /// Runs one update: the dice move, the camera renders, and a picture read back comes in.
    pub fn step(&mut self) -> Step {
        self.app.update();
        let world = self.app.world_mut();
        let landed = world.resource_mut::<Landed>().0.take();
        let fresh = std::mem::take(&mut world.resource_mut::<Shown>().fresh);
        let rolling = world.resource::<Throw>().rolling;
        let shown = world.resource::<Shown>();
        let current = !shown.pixels.is_empty() && (shown.width, shown.height) == (self.width, self.height);
        if (rolling || !current) && self.settle < SETTLE {
            self.settle = SETTLE;
        } else {
            self.settle = self.settle.saturating_sub(1);
        }
        Step { landed, fresh, busy: self.settle > 0 }
    }

    /// The last picture read back: its RGBA pixels, width, height and bytes per row.
    pub fn pixels(&self) -> Option<(&[u8], u32, u32, usize)> {
        let shown = self.app.world().resource::<Shown>();
        (!shown.pixels.is_empty()).then_some((shown.pixels.as_slice(), shown.width, shown.height, shown.row_bytes))
    }
}

/// The outward direction of a face in the die's own space. Opposite faces add up to 7.
fn normal_of(face: u8) -> Vec3 {
    match face {
        1 => Vec3::Y,
        6 => Vec3::NEG_Y,
        2 => Vec3::X,
        5 => Vec3::NEG_X,
        3 => Vec3::Z,
        _ => Vec3::NEG_Z,
    }
}

/// Where the pips of a face sit, across the face, in quarters of its width.
fn pips_of(face: u8) -> &'static [(f32, f32)] {
    const L: f32 = -0.25;
    const H: f32 = 0.25;
    match face {
        1 => &[(0.0, 0.0)],
        2 => &[(L, L), (H, H)],
        3 => &[(L, L), (0.0, 0.0), (H, H)],
        4 => &[(L, L), (L, H), (H, L), (H, H)],
        5 => &[(L, L), (L, H), (0.0, 0.0), (H, L), (H, H)],
        _ => &[(L, L), (L, 0.0), (L, H), (H, L), (H, 0.0), (H, H)],
    }
}

/// The turn that puts `face` on top, turned `yaw` about the table's up.
fn resting(face: u8, yaw: f32) -> Quat {
    Quat::from_rotation_y(yaw) * Quat::from_rotation_arc(normal_of(face), Vec3::Y)
}

/// Where die `index` of `count` rests on the table.
fn spot(index: usize, count: usize) -> Vec3 {
    let x = (index as f32 - (count as f32 - 1.0) / 2.0) * SPACING;
    Vec3::new(x, 0.5, 0.0)
}

/// Where the camera stands for a view.
fn eye_of(view: &View) -> Transform {
    // The original stand: 8.5 up and 6.5 back, about 52 degrees down.
    let down = 8.5f32.atan2(6.5);
    let at = Vec3::new(0.0, 0.0, 0.3);
    let back = Vec3::new(view.yaw.sin() * down.cos(), down.sin(), view.yaw.cos() * down.cos()) * view.distance;
    Transform::from_translation(at + back).looking_at(at, Vec3::Y)
}

/// The felt, the light, the camera with its readback and six dice, two of them shown.
fn set_table(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut random: ResMut<Random>,
    target: Res<Target>,
    view: Res<View>,
) {
    commands.spawn((Camera3d::default(), RenderTarget::from(target.image.clone()), eye_of(&view), Eye));
    commands.spawn(Readback::texture(target.image.clone())).observe(read_back);
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, shadow_maps_enabled: true, ..default() },
        Transform::from_xyz(4.0, 10.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((PointLight { intensity: 900_000.0, range: 30.0, ..default() }, Transform::from_xyz(-5.0, 6.0, 4.0)));

    let felt = materials.add(StandardMaterial { base_color: Color::srgb(0.08, 0.36, 0.20), perceptual_roughness: 0.95, ..default() });
    commands.spawn((Mesh3d(meshes.add(Plane3d::default().mesh().size(30.0, 20.0))), MeshMaterial3d(felt)));

    let body = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let ivory = materials.add(StandardMaterial { base_color: Color::srgb(0.96, 0.94, 0.88), perceptual_roughness: 0.35, ..default() });
    let pip = meshes.add(Cylinder::new(0.085, 0.03));
    let ink = materials.add(StandardMaterial { base_color: Color::srgb(0.08, 0.08, 0.10), perceptual_roughness: 0.6, ..default() });

    for index in 0..MOST {
        let face = random.face();
        let rest = resting(face, (random.unit() - 0.5) * 0.6);
        let at = spot(index, 2);
        commands
            .spawn((
                Die { index, face, from: at, to: at, rest, spin_axis: Vec3::X, spin: 0.0 },
                Mesh3d(body.clone()),
                MeshMaterial3d(ivory.clone()),
                Transform::from_translation(at).with_rotation(rest),
                if index < 2 { Visibility::Visible } else { Visibility::Hidden },
            ))
            .with_children(|die| {
                for face in 1..=6u8 {
                    let normal = normal_of(face);
                    // Two directions across the face.
                    let across = if normal.y.abs() > 0.5 { Vec3::X } else { Vec3::Y };
                    let along = normal.cross(across);
                    for (u, v) in pips_of(face) {
                        let at = normal * 0.5 + across * *u + along * *v;
                        die.spawn((
                            Mesh3d(pip.clone()),
                            MeshMaterial3d(ink.clone()),
                            Transform::from_translation(at).with_rotation(Quat::from_rotation_arc(Vec3::Y, normal)),
                        ));
                    }
                }
            });
    }
}

/// A picture came back from the GPU: kept when it has the target's size.
fn read_back(done: On<ReadbackComplete>, target: Res<Target>, mut shown: ResMut<Shown>) {
    let (width, height) = (target.width, target.height);
    let row = row_bytes(width);
    let data = &done.event().data;
    if data.len() < row * height as usize {
        // A picture of the size before a resize.
        return;
    }
    if data.get(3) == Some(&0) {
        // Nothing rendered yet: the felt's clear color is opaque.
        return;
    }
    shown.pixels.clear();
    shown.pixels.extend_from_slice(&data[..row * height as usize]);
    shown.width = width;
    shown.height = height;
    shown.row_bytes = row;
    shown.fresh = true;
}

/// A roll asked for: each shown die gets the face it lands on and a path there.
fn start_throw(mut asked: ResMut<RollAsked>, mut throw: ResMut<Throw>, count: Res<Count>, mut random: ResMut<Random>, mut dice: Query<&mut Die>) {
    if !std::mem::take(&mut asked.0) {
        return;
    }
    for mut die in &mut dice {
        if die.index >= count.0 {
            continue;
        }
        die.face = random.face();
        die.rest = resting(die.face, (random.unit() - 0.5) * 0.8);
        die.to = spot(die.index, count.0) + Vec3::new(0.0, 0.0, (random.unit() - 0.5) * 0.8);
        die.from = Vec3::new(die.to.x * 0.4 + (random.unit() - 0.5) * 2.0, 4.0, -4.0);
        die.spin_axis = Vec3::new(random.unit() - 0.5, random.unit() - 0.5, random.unit() - 0.5).normalize_or(Vec3::X);
        die.spin = (4.0 + random.unit() * 4.0) * PI;
    }
    throw.rolling = true;
    throw.elapsed = 0.0;
}

/// The dice in the air: falling, bouncing smaller each time, spinning to a stop on their face.
fn throw(time: Res<Time>, mut throw: ResMut<Throw>, count: Res<Count>, mut landed: ResMut<Landed>, mut dice: Query<(&Die, &mut Transform)>) {
    if !throw.rolling {
        return;
    }
    // A long pause between updates (the window was busy) moves the dice no more than a frame's worth.
    throw.elapsed += time.delta_secs().min(0.05);
    let mut all_down = true;
    for (die, mut transform) in &mut dice {
        if die.index >= count.0 {
            continue;
        }
        let length = THROW + die.index as f32 * STAGGER;
        let t = (throw.elapsed / length).min(1.0);
        if t < 1.0 {
            all_down = false;
        }
        let eased = 1.0 - (1.0 - t).powi(3);
        let mut at = die.from.lerp(die.to, eased);
        at.y = 0.5 + (die.from.y - 0.5) * (1.0 - t).powi(2) * (PI * 2.5 * t).cos().abs();
        transform.translation = at;
        transform.rotation = die.rest * Quat::from_axis_angle(die.spin_axis, die.spin * (1.0 - t).powi(3));
    }
    if all_down {
        throw.rolling = false;
        let mut shown: Vec<(usize, u8)> = dice.iter().filter(|(d, _)| d.index < count.0).map(|(d, _)| (d.index, d.face)).collect();
        shown.sort();
        let total: i64 = shown.iter().map(|(_, f)| *f as i64).sum();
        let faces: Vec<String> = shown.iter().map(|(_, f)| f.to_string()).collect();
        landed.0 = Some((total, faces.join(" + ")));
    }
}

/// A new count: as many dice shown, resting side by side.
fn lay_out(count: Res<Count>, throw: Res<Throw>, mut dice: Query<(&mut Die, &mut Transform, &mut Visibility)>) {
    if !count.is_changed() || throw.rolling {
        return;
    }
    for (mut die, mut transform, mut visibility) in &mut dice {
        let shown = die.index < count.0;
        *visibility = if shown { Visibility::Visible } else { Visibility::Hidden };
        if shown {
            die.to = spot(die.index, count.0);
            die.from = die.to;
            transform.translation = die.to;
            transform.rotation = die.rest;
        }
    }
}

/// The camera follows the view.
fn look(view: Res<View>, mut eye: Query<&mut Transform, With<Eye>>) {
    if !view.is_changed() {
        return;
    }
    for mut transform in &mut eye {
        *transform = eye_of(&view);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs updates until `done` says so, at most `most`; true when it did.
    fn until(scene: &mut Scene, most: usize, mut done: impl FnMut(&Step, &Scene) -> bool) -> bool {
        for _ in 0..most {
            let step = scene.step();
            if done(&step, scene) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        false
    }

    #[test]
    fn rows_are_padded_to_256_bytes() {
        assert_eq!(row_bytes(64), 256);
        assert_eq!(row_bytes(65), 512);
        assert_eq!(row_bytes(1), 256);
    }

    #[test]
    fn renders_headless_and_rolls_to_a_landing() {
        let mut scene = Scene::new(160, 120).expect("a renderer");
        assert!(until(&mut scene, 400, |_, s| s.pixels().is_some()), "a picture is read back");
        let (pixels, width, height, row) = scene.pixels().unwrap();
        assert_eq!((width, height, row), (160, 120, 768));
        // A corner shows the felt: green above red and blue.
        let corner = &pixels[0..4];
        assert!(corner[1] > corner[0] && corner[1] > corner[2], "felt green, got {corner:?}");
        assert_eq!(corner[3], 255);

        scene.set_count(3);
        scene.roll();
        let mut landed = None;
        assert!(until(&mut scene, 600, |step, _| {
            landed = step.landed.clone();
            landed.is_some()
        }));
        let (total, faces) = landed.unwrap();
        assert!((3..=18).contains(&total));
        assert_eq!(faces.split(" + ").count(), 3);
        assert!(until(&mut scene, 100, |step, _| !step.busy), "it settles");

        scene.resize(200, 100);
        assert!(until(&mut scene, 400, |_, s| s.pixels().is_some_and(|p| (p.1, p.2) == (200, 100))));
    }
}
