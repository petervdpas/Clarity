//! Custom part Dice: dice on a felt table, drawn in 3D with Bevy in the
//! native window Clarity gives it. Told to roll, the dice tumble, bounce and
//! land, then the part tells what they show. Keep the methods of `DicePart`
//! (src/clarity.rs), which follow the part's declaration.
//!
//! Bevy runs without a window loop of its own: the part host calls `frame`
//! about 60 times a second, and each call runs one `app.update()`.

mod clarity;
pub use clarity::*;
// Clarity's window, not Bevy's of the same name.
pub use clarity::Window;

use std::f32::consts::PI;

use bevy::app::PluginsState;
use bevy::prelude::*;
use bevy::window::{ExitCondition, PresentMode, PrimaryWindow, RawHandleWrapper, WindowResolution, WindowWrapper};
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};

/// The most dice the table holds.
const MOST: usize = 6;
/// How long a throw takes, in seconds, before the first die lands.
const THROW: f32 = 1.4;
/// How much later each next die lands.
const STAGGER: f32 = 0.12;
/// How far apart the dice rest.
const SPACING: f32 = 1.7;

pub struct Dice {
    host: DiceHost,
    app: App,
}

/// Clarity's window as Bevy keeps it: the part host's X11 window, which
/// outlives the part, and is drawn in on the host's thread only.
struct Surface(Window);

// The renderer runs on the part host's thread (pipelined rendering is off),
// so the window is never used from another one.
unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}

impl HasWindowHandle for Surface {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl HasDisplayHandle for Surface {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.0.display_handle()
    }
}

/// How many dice are on the table.
#[derive(Resource)]
struct Count(usize);

/// Code asked for a roll.
#[derive(Resource, Default)]
struct RollAsked(bool);

/// What the last throw showed, once every die has landed, for the part to tell.
#[derive(Resource, Default)]
struct Landed(Option<(i64, String)>);

/// The table's dice in throwing order, shown or not.
#[derive(Resource, Default)]
struct Throw {
    rolling: bool,
    elapsed: f32,
}

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

impl DicePart for Dice {
    fn create(window: Window, width: u32, height: u32, host: DiceHost) -> Self {
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .build()
                // The program has its own Ctrl+C.
                .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
                .set(bevy::window::WindowPlugin {
                    primary_window: Some(bevy::window::Window {
                        resolution: WindowResolution::new(width.max(1), height.max(1)).with_scale_factor_override(1.0),
                        present_mode: PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    exit_condition: ExitCondition::DontExit,
                    close_when_requested: false,
                    ..default()
                }),
        );
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1) | 1;
        app.insert_resource(ClearColor(Color::srgb(0.05, 0.16, 0.10)))
            .insert_resource(Count(2))
            .insert_resource(Random(seed))
            .init_resource::<RollAsked>()
            .init_resource::<Landed>()
            .init_resource::<Throw>()
            .add_systems(Startup, set_table)
            .add_systems(Update, (start_throw, throw, lay_out).chain());

        // Bevy draws in Clarity's window: the window Bevy made gets its handle.
        let mut primary = app.world_mut().query_filtered::<Entity, With<PrimaryWindow>>();
        let entity = primary.single(app.world()).expect("Bevy's window");
        let handle = RawHandleWrapper::new(&WindowWrapper::new(Surface(window))).expect("the window's handle");
        app.world_mut().entity_mut(entity).insert(handle);

        // As Bevy's own runner does before its first update.
        while app.plugins_state() == PluginsState::Adding {
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
        app.finish();
        app.cleanup();
        Dice { host, app }
    }

    fn resize(&mut self, width: u32, height: u32) {
        let world = self.app.world_mut();
        let mut windows = world.query_filtered::<&mut bevy::window::Window, With<PrimaryWindow>>();
        if let Ok(mut window) = windows.single_mut(world) {
            window.resolution.set_physical_resolution(width.max(1), height.max(1));
        }
    }

    fn frame(&mut self) {
        self.app.update();
        if let Some((total, faces)) = self.app.world_mut().resource_mut::<Landed>().0.take() {
            self.host.landed(total, &faces);
        }
    }

    fn set_count(&mut self, count: i64) {
        self.app.world_mut().resource_mut::<Count>().0 = count.clamp(1, MOST as i64) as usize;
    }

    fn roll(&mut self) {
        self.app.world_mut().resource_mut::<RollAsked>().0 = true;
    }
}

/// The felt, the light, the camera and six dice, two of them shown.
fn set_table(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut random: ResMut<Random>) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 8.5, 6.5).looking_at(Vec3::new(0.0, 0.0, 0.3), Vec3::Y)));
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

/// A roll asked for: each shown die gets the face it lands on and a path there.
fn start_throw(mut asked: ResMut<RollAsked>, mut throw: ResMut<Throw>, count: Res<Count>, mut random: ResMut<Random>, mut dice: Query<(&mut Die, &Transform)>) {
    if !std::mem::take(&mut asked.0) {
        return;
    }
    for (mut die, transform) in &mut dice {
        if die.index >= count.0 {
            continue;
        }
        die.face = random.face();
        die.rest = resting(die.face, (random.unit() - 0.5) * 0.8);
        die.to = spot(die.index, count.0) + Vec3::new(0.0, 0.0, (random.unit() - 0.5) * 0.8);
        die.from = Vec3::new(die.to.x * 0.4 + (random.unit() - 0.5) * 2.0, 4.0, -4.0);
        die.spin_axis = Vec3::new(random.unit() - 0.5, random.unit() - 0.5, random.unit() - 0.5).normalize_or(Vec3::X);
        die.spin = (4.0 + random.unit() * 4.0) * PI;
        let _ = transform;
    }
    throw.rolling = true;
    throw.elapsed = 0.0;
}

/// The dice in the air: falling, bouncing smaller each time, spinning to a stop on their face.
fn throw(time: Res<Time>, mut throw: ResMut<Throw>, count: Res<Count>, mut landed: ResMut<Landed>, mut dice: Query<(&Die, &mut Transform)>) {
    if !throw.rolling {
        return;
    }
    throw.elapsed += time.delta_secs();
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
