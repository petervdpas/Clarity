//! Where a graph's nodes stand (docs/GRAPH-PLAN.md), in the graph's own
//! units around 0,0, which the part fits into its place. Every layout
//! places the same graph the same way each time: no randomness, a fixed
//! number of steps, ties broken by the nodes' order.

use std::collections::VecDeque;

/// How far apart nodes stand, in the graph's units.
pub const SPACING: f64 = 90.0;

/// How a graph places its nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// Pushed apart, pulled together along their edges.
    #[default]
    Force,
    /// In layers along the edges' direction, top to bottom.
    Layered,
    /// In rings around one node, by how many edges away they are.
    Radial,
    /// On one circle, in their order.
    Circle,
}

impl Layout {
    pub fn read(word: Option<&str>) -> Layout {
        match word {
            Some("layered") => Layout::Layered,
            Some("radial") => Layout::Radial,
            Some("circle") => Layout::Circle,
            _ => Layout::Force,
        }
    }
}

/// The places of `count` nodes joined by `edges` (from, to), laid out as
/// `layout`; a radial layout puts `focus` (the first node without one) in
/// the middle.
pub fn place(layout: Layout, count: usize, edges: &[(usize, usize)], focus: Option<usize>) -> Vec<(f64, f64)> {
    let edges: Vec<(usize, usize)> = edges.iter().copied().filter(|(a, b)| *a < count && *b < count).collect();
    match layout {
        Layout::Circle => circle(count),
        Layout::Radial => radial(count, &edges, focus.filter(|f| *f < count).unwrap_or(0)),
        Layout::Layered => layered(count, &edges),
        Layout::Force => packed(count, &edges),
    }
}

/// The groups of nodes joined to each other, each in the nodes' order, the groups in the order of their first node.
fn groups(count: usize, edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let next = neighbours(count, edges);
    let mut group = vec![usize::MAX; count];
    let mut out: Vec<Vec<usize>> = Vec::new();
    for start in 0..count {
        if group[start] != usize::MAX {
            continue;
        }
        let mut members = vec![start];
        group[start] = out.len();
        let mut at = 0;
        while at < members.len() {
            for &n in &next[members[at]] {
                if group[n] == usize::MAX {
                    group[n] = out.len();
                    members.push(n);
                }
            }
            at += 1;
        }
        members.sort_unstable();
        out.push(members);
    }
    out
}

/// Each group of joined nodes laid out by force on its own, the groups
/// then packed in rows, largest first, so groups that share no edge do
/// not push each other out of sight.
fn packed(count: usize, edges: &[(usize, usize)]) -> Vec<(f64, f64)> {
    let mut groups = groups(count, edges);
    if groups.len() <= 1 {
        return force(count, edges);
    }
    // Largest first; groups of one size keep their order.
    groups.sort_by_key(|g| std::cmp::Reverse(g.len()));
    let laid: Vec<(Vec<usize>, Vec<(f64, f64)>, (f64, f64, f64, f64))> = groups
        .into_iter()
        .map(|members| {
            let local: Vec<(usize, usize)> = edges
                .iter()
                .filter_map(|(a, b)| Some((members.iter().position(|m| m == a)?, members.iter().position(|m| m == b)?)))
                .collect();
            let places = force(members.len(), &local);
            let (mut left, mut top, mut right, mut bottom) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for p in &places {
                left = left.min(p.0);
                top = top.min(p.1);
                right = right.max(p.0);
                bottom = bottom.max(p.1);
            }
            (members, places, (left, top, right, bottom))
        })
        .collect();
    let area: f64 = laid.iter().map(|(_, _, b)| (b.2 - b.0 + SPACING) * (b.3 - b.1 + SPACING)).sum();
    let row_width = area.sqrt().max(SPACING) * 1.4;
    let mut places = vec![(0.0, 0.0); count];
    let (mut x, mut y, mut row_height) = (0.0f64, 0.0f64, 0.0f64);
    for (members, local, (left, top, right, bottom)) in laid {
        let (width, height) = (right - left + SPACING, bottom - top + SPACING);
        if x > 0.0 && x + width > row_width {
            x = 0.0;
            y += row_height;
            row_height = 0.0;
        }
        for (m, p) in members.iter().zip(local) {
            places[*m] = (x + p.0 - left, y + p.1 - top);
        }
        x += width;
        row_height = row_height.max(height);
    }
    places
}

/// On one circle from the top, clockwise, far enough round for each to have its room.
fn circle(count: usize) -> Vec<(f64, f64)> {
    if count == 1 {
        return vec![(0.0, 0.0)];
    }
    let radius = (count as f64 * SPACING / std::f64::consts::TAU).max(SPACING);
    (0..count)
        .map(|i| {
            let angle = i as f64 / count as f64 * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
            (radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

/// The nodes next to each, either way along the edges, in the edges' order.
fn neighbours(count: usize, edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let mut next = vec![Vec::new(); count];
    for &(a, b) in edges {
        if a != b {
            if !next[a].contains(&b) {
                next[a].push(b);
            }
            if !next[b].contains(&a) {
                next[b].push(a);
            }
        }
    }
    next
}

/// The focus in the middle, each ring further out one edge further away,
/// a ring's nodes in the order they were reached, spread round it; nodes
/// that cannot be reached stand in a ring outside the others.
fn radial(count: usize, edges: &[(usize, usize)], focus: usize) -> Vec<(f64, f64)> {
    if count == 0 {
        return Vec::new();
    }
    let next = neighbours(count, edges);
    let mut ring = vec![usize::MAX; count];
    let mut order = Vec::with_capacity(count);
    let mut queue = VecDeque::from([focus]);
    ring[focus] = 0;
    while let Some(node) = queue.pop_front() {
        order.push(node);
        for &n in &next[node] {
            if ring[n] == usize::MAX {
                ring[n] = ring[node] + 1;
                queue.push_back(n);
            }
        }
    }
    let outer = order.iter().map(|n| ring[*n]).max().unwrap_or(0) + 1;
    for (node, r) in ring.iter_mut().enumerate() {
        if *r == usize::MAX {
            *r = outer;
            order.push(node);
        }
    }
    let rings = ring.iter().copied().max().unwrap_or(0);
    let mut places = vec![(0.0, 0.0); count];
    for r in 1..=rings {
        let members: Vec<usize> = order.iter().copied().filter(|n| ring[*n] == r).collect();
        let radius = (r as f64 * SPACING * 1.4).max(members.len() as f64 * SPACING / std::f64::consts::TAU);
        for (i, node) in members.iter().enumerate() {
            let angle = i as f64 / members.len() as f64 * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
            places[*node] = (radius * angle.cos(), radius * angle.sin());
        }
    }
    places
}

/// Each node one layer below the furthest node with an edge to it (an
/// edge that goes back round a cycle does not count), the nodes of a layer
/// put in order of where their neighbours stand, in a few sweeps.
fn layered(count: usize, edges: &[(usize, usize)]) -> Vec<(f64, f64)> {
    if count == 0 {
        return Vec::new();
    }
    let forward = acyclic(count, edges);
    // The longest way down to each node, taking the nodes in an order where every edge points down.
    let mut layer = vec![0usize; count];
    for &node in &topological(count, &forward) {
        for &(a, b) in &forward {
            if a == node {
                layer[b] = layer[b].max(layer[a] + 1);
            }
        }
    }
    // A node nothing points to stands just above the highest node it points to, not at the top.
    for node in 0..count {
        let pointed_at = forward.iter().any(|(_, b)| *b == node);
        let below = forward.iter().filter(|(a, _)| *a == node).map(|(_, b)| layer[*b]).min();
        if let (false, Some(below)) = (pointed_at, below) {
            layer[node] = below.saturating_sub(1);
        }
    }
    let layers = layer.iter().copied().max().unwrap_or(0) + 1;
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); layers];
    for node in 0..count {
        rows[layer[node]].push(node);
    }
    let next = neighbours(count, edges);
    let mut at = vec![0.0f64; count];
    let number = |rows: &[Vec<usize>], at: &mut [f64]| {
        for row in rows {
            for (i, n) in row.iter().enumerate() {
                at[*n] = i as f64;
            }
        }
    };
    number(&rows, &mut at);
    for sweep in 0..8 {
        let order: Vec<usize> = if sweep % 2 == 0 { (1..layers).collect() } else { (0..layers.saturating_sub(1)).rev().collect() };
        for l in order {
            // Going down a layer's nodes follow their neighbours in the layer above; going up, below.
            let beside = if sweep % 2 == 0 { l.checked_sub(1) } else { Some(l + 1) };
            let mut keyed: Vec<(f64, usize, usize)> = rows[l]
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    let seen: Vec<f64> = next[*n].iter().filter(|m| Some(layer[**m]) == beside).map(|m| at[*m]).collect();
                    let key = if seen.is_empty() { at[*n] } else { seen.iter().sum::<f64>() / seen.len() as f64 };
                    (key, i, *n)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            rows[l] = keyed.into_iter().map(|(_, _, n)| n).collect();
            number(&rows, &mut at);
        }
    }
    let mut places = vec![(0.0, 0.0); count];
    for (l, row) in rows.iter().enumerate() {
        let width = (row.len().saturating_sub(1)) as f64 * SPACING * 1.3;
        for (i, n) in row.iter().enumerate() {
            places[*n] = (i as f64 * SPACING * 1.3 - width / 2.0, l as f64 * SPACING);
        }
    }
    places
}

/// The edges without the ones that close a cycle, found walking the nodes
/// depth first in their order; an edge from a node to itself goes too.
fn acyclic(count: usize, edges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    // 0: not reached, 1: on the way, 2: done.
    let mut state = vec![0u8; count];
    let mut kept = Vec::new();
    let mut out: Vec<Vec<(usize, usize)>> = vec![Vec::new(); count];
    for (i, &(a, b)) in edges.iter().enumerate() {
        out[a].push((b, i));
    }
    let mut keep = vec![false; edges.len()];
    for start in 0..count {
        if state[start] != 0 {
            continue;
        }
        let mut stack = vec![(start, 0usize)];
        state[start] = 1;
        while let Some(&mut (node, ref mut next)) = stack.last_mut() {
            if let Some(&(to, edge)) = out[node].get(*next) {
                *next += 1;
                match state[to] {
                    0 => {
                        keep[edge] = true;
                        state[to] = 1;
                        stack.push((to, 0));
                    }
                    2 => keep[edge] = true,
                    _ => {}
                }
            } else {
                state[node] = 2;
                stack.pop();
            }
        }
    }
    for (i, e) in edges.iter().enumerate() {
        if keep[i] {
            kept.push(*e);
        }
    }
    kept
}

/// The nodes in an order where every edge goes from an earlier to a later one (the edges have no cycle).
fn topological(count: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut into = vec![0usize; count];
    for &(_, b) in edges {
        into[b] += 1;
    }
    let mut ready: VecDeque<usize> = (0..count).filter(|n| into[*n] == 0).collect();
    let mut order = Vec::with_capacity(count);
    while let Some(n) = ready.pop_front() {
        order.push(n);
        for &(a, b) in edges {
            if a == n {
                into[b] -= 1;
                if into[b] == 0 {
                    ready.push_back(b);
                }
            }
        }
    }
    order
}

/// Pushed apart, pulled together along their edges and a little toward
/// the middle, from their places on a circle, in a fixed number of steps
/// that move less each time.
fn force(count: usize, edges: &[(usize, usize)]) -> Vec<(f64, f64)> {
    let mut places = circle(count);
    if count < 2 {
        return places;
    }
    let steps = if count > 300 { 60 } else if count > 100 { 150 } else { 300 };
    let k = SPACING;
    let mut reach = SPACING * (count as f64).sqrt();
    let cool = reach / steps as f64;
    let mut moved = vec![(0.0f64, 0.0f64); count];
    for _ in 0..steps {
        moved.iter_mut().for_each(|m| *m = (0.0, 0.0));
        for a in 0..count {
            for b in a + 1..count {
                let (dx, dy) = (places[a].0 - places[b].0, places[a].1 - places[b].1);
                let d2 = (dx * dx + dy * dy).max(0.01);
                let push = k * k / d2;
                moved[a].0 += dx * push;
                moved[a].1 += dy * push;
                moved[b].0 -= dx * push;
                moved[b].1 -= dy * push;
            }
        }
        for &(a, b) in edges {
            if a == b {
                continue;
            }
            let (dx, dy) = (places[a].0 - places[b].0, places[a].1 - places[b].1);
            let d = (dx * dx + dy * dy).sqrt().max(0.1);
            let pull = d / k;
            moved[a].0 -= dx * pull;
            moved[a].1 -= dy * pull;
            moved[b].0 += dx * pull;
            moved[b].1 += dy * pull;
        }
        for (place, m) in places.iter_mut().zip(&moved) {
            let gravity = 0.02;
            let (mx, my) = (m.0 - place.0 * gravity, m.1 - place.1 * gravity);
            let length = (mx * mx + my * my).sqrt();
            if length > 0.0 {
                let step = length.min(reach);
                place.0 += mx / length * step;
                place.1 += my / length * step;
            }
        }
        reach = (reach - cool).max(1.0);
    }
    places
}

#[cfg(test)]
mod tests {
    use super::*;

    fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
        ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
    }

    const TRIANGLE_AND_TAIL: [(usize, usize); 4] = [(0, 1), (1, 2), (2, 0), (2, 3)];

    #[test]
    fn every_layout_places_every_node_and_the_same_way_each_time() {
        for layout in [Layout::Force, Layout::Layered, Layout::Radial, Layout::Circle] {
            let once = place(layout, 5, &TRIANGLE_AND_TAIL, None);
            assert_eq!(once.len(), 5, "{layout:?}");
            assert!(once.iter().all(|(x, y)| x.is_finite() && y.is_finite()), "{layout:?}");
            assert_eq!(once, place(layout, 5, &TRIANGLE_AND_TAIL, None), "{layout:?}");
            for a in 0..5 {
                for b in a + 1..5 {
                    assert!(distance(once[a], once[b]) > 10.0, "{layout:?}: {a} and {b} stand on each other");
                }
            }
        }
    }

    #[test]
    fn no_nodes_and_one_node_are_placed_too() {
        for layout in [Layout::Force, Layout::Layered, Layout::Radial, Layout::Circle] {
            assert!(place(layout, 0, &[], None).is_empty());
            assert_eq!(place(layout, 1, &[(0, 0)], None).len(), 1);
        }
    }

    #[test]
    fn edges_to_nodes_that_are_not_there_are_left_out() {
        assert_eq!(place(Layout::Layered, 2, &[(0, 1), (1, 7)], None).len(), 2);
    }

    #[test]
    fn layers_follow_the_edges_down_and_a_cycle_does_not_stop_them() {
        let places = place(Layout::Layered, 4, &TRIANGLE_AND_TAIL, None);
        assert!(places[0].1 < places[1].1 && places[1].1 < places[2].1 && places[2].1 < places[3].1, "{places:?}");
    }

    #[test]
    fn a_node_nothing_points_to_stands_just_above_what_it_points_to() {
        // 0 -> 1 -> 2, and 3 -> 2: 3 stands beside 1, not beside 0.
        let places = place(Layout::Layered, 4, &[(0, 1), (1, 2), (3, 2)], None);
        assert_eq!(places[3].1, places[1].1, "{places:?}");
    }

    #[test]
    fn a_radial_layout_puts_the_focus_in_the_middle_and_rings_by_distance() {
        let places = place(Layout::Radial, 5, &[(0, 1), (1, 2), (3, 0)], Some(1));
        assert_eq!(places[1], (0.0, 0.0));
        let from = |n: usize| distance(places[n], (0.0, 0.0));
        assert!((from(0) - from(2)).abs() < 1e-9, "one edge away, one ring");
        assert!(from(3) > from(0), "two edges away, further out");
        assert!(from(4) > from(3), "not reached, outside the rest");
    }

    #[test]
    fn groups_that_share_no_edge_stand_near_each_other() {
        // Two pairs and a node on its own: no group is pushed out of sight.
        let places = place(Layout::Force, 5, &[(0, 1), (2, 3)], None);
        let (left, right) = places.iter().fold((f64::MAX, f64::MIN), |(l, r), p| (l.min(p.0), r.max(p.0)));
        let (top, bottom) = places.iter().fold((f64::MAX, f64::MIN), |(t, b), p| (t.min(p.1), b.max(p.1)));
        assert!(right - left < SPACING * 5.0 && bottom - top < SPACING * 5.0, "{places:?}");
        for a in 0..5 {
            for b in a + 1..5 {
                assert!(distance(places[a], places[b]) > SPACING * 0.5, "{a} and {b}: {places:?}");
            }
        }
    }

    #[test]
    fn a_force_layout_keeps_joined_nodes_closer_than_others() {
        let places = place(Layout::Force, 6, &[(0, 1), (1, 2), (3, 4), (4, 5)], None);
        assert!(distance(places[0], places[1]) < distance(places[0], places[4]), "{places:?}");
    }
}
