//! Arranging a diagram's boxes, as layered diagram tools do:
//!
//! 1. The boxes fall apart into groups that link to each other; each group
//!    is laid out on its own, the groups then side by side, the largest
//!    first, wrapping at the canvas's width. Boxes without links come last.
//! 2. In a group, a box stands a row above the boxes it links to (a holder
//!    above what it holds, a key above the table it points at); a box no
//!    one links to stands just above the first box it links to, so lines
//!    stay short. Cycles are broken where they are met.
//! 3. A line that skips rows gets a waypoint in each row it passes, so the
//!    rows are ordered with it in mind and it bends around the boxes: those
//!    waypoints are the line's bend points (`via`).
//! 4. Each row is ordered by the median place of its neighbours, sweeping
//!    down and up, with neighbours swapped where that crosses fewer lines;
//!    the order that crosses fewest is kept.
//! 5. Each box is pulled across toward its neighbours, never over another.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// A box to place: its id, its size, and where it ends up.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutBox {
    pub id: i64,
    pub width: i64,
    pub height: i64,
    #[serde(default)]
    pub x: i64,
    #[serde(default)]
    pub y: i64,
}

/// A line from one box to another, by their ids; `key` is the caller's own
/// (a property's id), `via` the bend points it gets: x, y, x, y, ...
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutLink {
    pub from: i64,
    pub to: i64,
    #[serde(default)]
    pub key: i64,
    #[serde(default)]
    pub via: Vec<i64>,
}

const GAP_X: f64 = 60.0;
const GAP_Y: f64 = 120.0;
const MARGIN: f64 = 40.0;
const GROUP_GAP: f64 = 160.0;
/// How wide a line's waypoint is in its row.
const WAYPOINT: f64 = 24.0;
const LOOSE_PER_ROW: usize = 6;
const ORDER_ROUNDS: usize = 24;
const PULL_ROUNDS: usize = 30;

/// The boxes, placed: on the grid of `grid` units (none when 0 or less),
/// groups wrapping where they would pass `width` (none when 0 or less).
pub fn arrange(boxes: &[LayoutBox], links: &[LayoutLink], grid: i64, width: i64) -> Vec<LayoutBox> {
    layout(boxes, links, grid, width).0
}

/// The links with their bend points, as `arrange` places the boxes.
pub fn route(boxes: &[LayoutBox], links: &[LayoutLink], grid: i64, width: i64) -> Vec<LayoutLink> {
    layout(boxes, links, grid, width).1
}

/// A node of a group: a box, or a waypoint of a line that skips rows.
#[derive(Clone)]
struct Node {
    box_id: Option<i64>,
    width: f64,
    height: f64,
    row: usize,
    x: f64,
}

struct Group {
    nodes: Vec<Node>,
    /// Edges between nodes in adjacent rows, upper to lower.
    edges: Vec<(usize, usize)>,
    rows: Vec<Vec<usize>>,
    /// For each link of the caller: its chain of nodes, upper to lower, and whether it runs upward.
    chains: Vec<(usize, Vec<usize>, bool)>,
}

fn layout(boxes: &[LayoutBox], links: &[LayoutLink], grid: i64, width: i64) -> (Vec<LayoutBox>, Vec<LayoutLink>) {
    let ids: BTreeSet<i64> = boxes.iter().map(|b| b.id).collect();
    let size: BTreeMap<i64, (f64, f64)> = boxes.iter().map(|b| (b.id, (b.width.max(1) as f64, b.height.max(1) as f64))).collect();
    let valid: Vec<(usize, i64, i64)> = links.iter().enumerate().filter(|(_, l)| l.from != l.to && ids.contains(&l.from) && ids.contains(&l.to)).map(|(i, l)| (i, l.from, l.to)).collect();
    // The groups: boxes that link to each other, however.
    let mut near: BTreeMap<i64, BTreeSet<i64>> = BTreeMap::new();
    for &(_, f, t) in &valid {
        near.entry(f).or_default().insert(t);
        near.entry(t).or_default().insert(f);
    }
    let mut seen: BTreeSet<i64> = BTreeSet::new();
    let mut groups: Vec<Vec<i64>> = Vec::new();
    let mut loose: Vec<i64> = Vec::new();
    for b in boxes {
        if seen.contains(&b.id) {
            continue;
        }
        if !near.contains_key(&b.id) {
            loose.push(b.id);
            seen.insert(b.id);
            continue;
        }
        let mut members = Vec::new();
        let mut queue = VecDeque::from([b.id]);
        seen.insert(b.id);
        while let Some(n) = queue.pop_front() {
            members.push(n);
            for &m in near.get(&n).into_iter().flatten() {
                if seen.insert(m) {
                    queue.push_back(m);
                }
            }
        }
        groups.push(members);
    }
    // Each group laid out on its own, at its own 0, 0.
    let mut laid: Vec<(Group, f64, f64)> = groups
        .iter()
        .map(|members| {
            let set: BTreeSet<i64> = members.iter().copied().collect();
            let own: Vec<(usize, i64, i64)> = valid.iter().filter(|(_, f, _)| set.contains(f)).copied().collect();
            let mut g = build(members, &own, &size);
            order(&mut g);
            pull(&mut g);
            let (w, h) = extent(&g);
            (g, w, h)
        })
        .collect();
    laid.sort_by(|a, b| (b.1 * b.2).partial_cmp(&(a.1 * a.2)).unwrap_or(std::cmp::Ordering::Equal));
    // The groups side by side, wrapping at the width; then the loose boxes in rows.
    let room = if width > 0 { width as f64 - 2.0 * MARGIN } else { f64::MAX };
    let snap = |v: f64| -> i64 {
        let v = v.round() as i64;
        if grid > 0 { (v + grid / 2) / grid * grid } else { v }
    };
    let mut placed: BTreeMap<i64, (i64, i64)> = BTreeMap::new();
    let mut routed: BTreeMap<usize, Vec<i64>> = BTreeMap::new();
    let (mut x, mut y, mut tallest) = (MARGIN, MARGIN, 0.0f64);
    for (g, w, h) in &laid {
        if x > MARGIN && x - MARGIN + w > room {
            x = MARGIN;
            y += tallest + GROUP_GAP;
            tallest = 0.0;
        }
        let tops = row_tops(g);
        for n in &g.nodes {
            if let Some(id) = n.box_id {
                placed.insert(id, (snap(x + n.x), snap(y + tops[n.row])));
            }
        }
        for (link, chain, upward) in &g.chains {
            let mut via: Vec<i64> = Vec::new();
            let mut points: Vec<usize> = chain.iter().copied().filter(|&n| g.nodes[n].box_id.is_none()).collect();
            if *upward {
                points.reverse();
            }
            // Through each row it skips straight down its gap: in at the row's top, out at its bottom.
            for n in points {
                let node = &g.nodes[n];
                let across = snap(x + node.x + node.width / 2.0);
                let (top, bottom) = (snap(y + tops[node.row]), snap(y + tops[node.row] + row_height(g, node.row)));
                let (first, second) = if *upward { (bottom, top) } else { (top, bottom) };
                via.extend([across, first, across, second]);
            }
            routed.insert(*link, via);
        }
        x += w + GROUP_GAP;
        tallest = tallest.max(*h);
    }
    if !loose.is_empty() {
        let mut row_y = if placed.is_empty() { MARGIN } else { y + tallest + GROUP_GAP };
        for chunk in loose.chunks(LOOSE_PER_ROW) {
            let mut lx = MARGIN;
            let mut high = 0.0f64;
            for id in chunk {
                let (w, h) = size[id];
                placed.insert(*id, (snap(lx), snap(row_y)));
                lx += w + GAP_X;
                high = high.max(h);
            }
            row_y += high + GAP_Y;
        }
    }
    let out_boxes = boxes
        .iter()
        .map(|b| {
            let (x, y) = placed.get(&b.id).copied().unwrap_or((b.x, b.y));
            LayoutBox { x, y, ..b.clone() }
        })
        .collect();
    let out_links = links.iter().enumerate().map(|(i, l)| LayoutLink { via: routed.get(&i).cloned().unwrap_or_default(), ..l.clone() }).collect();
    (out_boxes, out_links)
}

/// A group's nodes in rows: cycles broken, rows by longest path and pulled
/// down toward their links, waypoints in the rows a line skips.
fn build(members: &[i64], links: &[(usize, i64, i64)], size: &BTreeMap<i64, (f64, f64)>) -> Group {
    let index: BTreeMap<i64, usize> = members.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    let n = members.len();
    let mut out: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for &(_, f, t) in links {
        out[index[&f]].insert(index[&t]);
    }
    // Break cycles: a link back to a box on the walk's path runs the other way.
    let mut state = vec![0u8; n];
    let mut reversed: BTreeSet<(usize, usize)> = BTreeSet::new();
    fn walk(v: usize, out: &[BTreeSet<usize>], state: &mut [u8], reversed: &mut BTreeSet<(usize, usize)>) {
        state[v] = 1;
        for &w in &out[v] {
            match state[w] {
                0 => walk(w, out, state, reversed),
                1 => {
                    reversed.insert((v, w));
                }
                _ => {}
            }
        }
        state[v] = 2;
    }
    for v in 0..n {
        if state[v] == 0 {
            walk(v, &out, &mut state, &mut reversed);
        }
    }
    let mut down: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for v in 0..n {
        for &w in &out[v] {
            if reversed.contains(&(v, w)) {
                if !out[w].contains(&v) {
                    down[w].insert(v);
                }
            } else {
                down[v].insert(w);
            }
        }
    }
    // Rows: one below the lowest box above.
    let mut row = vec![0usize; n];
    loop {
        let mut moved = false;
        for v in 0..n {
            for &w in &down[v] {
                if row[w] < row[v] + 1 {
                    row[w] = row[v] + 1;
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }
    // A box nothing links down to stands just above the nearest box it links to.
    let has_up: Vec<bool> = (0..n).map(|v| down.iter().any(|d| d.contains(&v))).collect();
    for v in 0..n {
        if !has_up[v] {
            if let Some(nearest) = down[v].iter().map(|&w| row[w]).min() {
                row[v] = nearest.saturating_sub(1);
            }
        }
    }
    let mut nodes: Vec<Node> = members.iter().enumerate().map(|(i, id)| Node { box_id: Some(*id), width: size[id].0, height: size[id].1, row: row[i], x: 0.0 }).collect();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut chains: Vec<(usize, Vec<usize>, bool)> = Vec::new();
    // Each link a chain of unit edges, with a waypoint in each row it skips; links of the same pair share it.
    let mut shared: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for &(link, f, t) in links {
        let (a, b) = (index[&f], index[&t]);
        let (upper, lower, upward) = if row[a] <= row[b] { (a, b, false) } else { (b, a, true) };
        if row[upper] == row[lower] {
            chains.push((link, vec![upper, lower], upward));
            continue;
        }
        let chain = shared
            .entry((upper, lower))
            .or_insert_with(|| {
                let mut chain = vec![upper];
                let mut previous = upper;
                for r in row[upper] + 1..row[lower] {
                    nodes.push(Node { box_id: None, width: WAYPOINT, height: 0.0, row: r, x: 0.0 });
                    let id = nodes.len() - 1;
                    edges.push((previous, id));
                    chain.push(id);
                    previous = id;
                }
                edges.push((previous, lower));
                chain.push(lower);
                chain
            })
            .clone();
        chains.push((link, chain, upward));
    }
    let count = nodes.iter().map(|n| n.row).max().map_or(0, |m| m + 1);
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (i, node) in nodes.iter().enumerate() {
        rows[node.row].push(i);
    }
    Group { nodes, edges, rows, chains }
}

/// The places of a row's nodes, by node.
fn places(rows: &[Vec<usize>], count: usize) -> Vec<f64> {
    let mut at = vec![0.0; count];
    for row in rows {
        for (i, &n) in row.iter().enumerate() {
            at[n] = i as f64;
        }
    }
    at
}

/// How many times the lines between rows `r` and `r + 1` cross.
fn crossings_between(g: &Group, rows: &[Vec<usize>], r: usize, at: &[f64]) -> usize {
    let mut pairs: Vec<(f64, f64)> = g.edges.iter().filter(|(u, v)| g.nodes[*u].row == r && g.nodes[*v].row == r + 1 && rows[r].contains(u)).map(|(u, v)| (at[*u], at[*v])).collect();
    pairs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut crossed = 0;
    for i in 0..pairs.len() {
        for j in i + 1..pairs.len() {
            if pairs[i].0 < pairs[j].0 && pairs[i].1 > pairs[j].1 {
                crossed += 1;
            }
        }
    }
    crossed
}

fn crossings(g: &Group, rows: &[Vec<usize>]) -> usize {
    let at = places(rows, g.nodes.len());
    (0..rows.len().saturating_sub(1)).map(|r| crossings_between(g, rows, r, &at)).sum()
}

/// Rows ordered by the median of their neighbours' places, down and up, neighbours swapped where fewer lines cross.
fn order(g: &mut Group) {
    let n = g.nodes.len();
    let mut up: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut below: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(u, v) in &g.edges {
        below[u].push(v);
        up[v].push(u);
    }
    let mut best = g.rows.clone();
    let mut fewest = crossings(g, &best);
    let mut rows = g.rows.clone();
    for round in 0..ORDER_ROUNDS {
        let downward = round % 2 == 0;
        let sweep: Vec<usize> = if downward { (1..rows.len()).collect() } else { (0..rows.len().saturating_sub(1)).rev().collect() };
        for r in sweep {
            let at = places(&rows, n);
            let mut keyed: Vec<(f64, usize, usize)> = rows[r]
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let near = if downward { &up[v] } else { &below[v] };
                    let mut spots: Vec<f64> = near.iter().map(|&w| at[w]).collect();
                    spots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let key = match spots.len() {
                        0 => i as f64,
                        k if k % 2 == 1 => spots[k / 2],
                        k => (spots[k / 2 - 1] + spots[k / 2]) / 2.0,
                    };
                    (key, i, v)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)));
            rows[r] = keyed.into_iter().map(|(_, _, v)| v).collect();
        }
        // Swap neighbours while it crosses fewer lines.
        let mut better = true;
        while better {
            better = false;
            for r in 0..rows.len() {
                for i in 0..rows[r].len().saturating_sub(1) {
                    let before = crossings(g, &rows);
                    rows[r].swap(i, i + 1);
                    if crossings(g, &rows) < before {
                        better = true;
                    } else {
                        rows[r].swap(i, i + 1);
                    }
                }
            }
        }
        let crossed = crossings(g, &rows);
        if crossed < fewest {
            fewest = crossed;
            best = rows.clone();
        }
        if fewest == 0 {
            break;
        }
    }
    g.rows = best;
}

/// Each node pulled across toward the middle of its neighbours, the rows kept in order and apart.
fn pull(g: &mut Group) {
    let n = g.nodes.len();
    let mut near: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(u, v) in &g.edges {
        near[u].push(v);
        near[v].push(u);
    }
    // Start packed from the left.
    for row in &g.rows {
        let mut x = 0.0;
        for &v in row {
            g.nodes[v].x = x;
            x += g.nodes[v].width + GAP_X;
        }
    }
    let middle = |g: &Group, v: usize| g.nodes[v].x + g.nodes[v].width / 2.0;
    for _ in 0..PULL_ROUNDS {
        for r in 0..g.rows.len() {
            let row = g.rows[r].clone();
            let wanted: Vec<f64> = row
                .iter()
                .map(|&v| {
                    let spots: Vec<f64> = near[v].iter().map(|&w| middle(g, w)).collect();
                    if spots.is_empty() { g.nodes[v].x } else { spots.iter().sum::<f64>() / spots.len() as f64 - g.nodes[v].width / 2.0 }
                })
                .collect();
            // Left to right: no node over the one before it; right to left: none over the one after; the two averaged.
            let mut right = wanted.clone();
            for i in 1..row.len() {
                let least = right[i - 1] + g.nodes[row[i - 1]].width + GAP_X;
                if right[i] < least {
                    right[i] = least;
                }
            }
            let mut left = wanted.clone();
            for i in (0..row.len().saturating_sub(1)).rev() {
                let most = left[i + 1] - g.nodes[row[i]].width - GAP_X;
                if left[i] > most {
                    left[i] = most;
                }
            }
            let mut xs: Vec<f64> = (0..row.len()).map(|i| (right[i] + left[i]) / 2.0).collect();
            for i in 1..row.len() {
                let least = xs[i - 1] + g.nodes[row[i - 1]].width + GAP_X;
                if xs[i] < least {
                    xs[i] = least;
                }
            }
            for (i, &v) in row.iter().enumerate() {
                g.nodes[v].x = xs[i];
            }
        }
    }
    // The group starts at 0.
    let least = g.nodes.iter().map(|n| n.x).fold(f64::MAX, f64::min);
    if least.is_finite() {
        for node in &mut g.nodes {
            node.x -= least;
        }
    }
}

/// How high a row is: its tallest box.
fn row_height(g: &Group, row: usize) -> f64 {
    g.rows[row].iter().map(|&v| g.nodes[v].height).fold(0.0, f64::max)
}

/// Where each row starts down, from 0.
fn row_tops(g: &Group) -> Vec<f64> {
    let mut tops = Vec::with_capacity(g.rows.len());
    let mut y = 0.0;
    for row in &g.rows {
        tops.push(y);
        let tallest = row.iter().map(|&v| g.nodes[v].height).fold(0.0, f64::max);
        y += tallest + GAP_Y;
    }
    tops
}

/// How wide and high a laid-out group is.
fn extent(g: &Group) -> (f64, f64) {
    let width = g.nodes.iter().map(|n| n.x + n.width).fold(0.0, f64::max);
    let tops = row_tops(g);
    let height = g.rows.iter().enumerate().map(|(r, row)| tops[r] + row.iter().map(|&v| g.nodes[v].height).fold(0.0, f64::max)).fold(0.0, f64::max);
    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(id: i64) -> LayoutBox {
        LayoutBox { id, width: 200, height: 100, x: 0, y: 0 }
    }

    fn l(from: i64, to: i64) -> LayoutLink {
        LayoutLink { from, to, key: from * 100 + to, via: Vec::new() }
    }

    fn at(placed: &[LayoutBox], id: i64) -> (i64, i64) {
        placed.iter().find(|p| p.id == id).map(|p| (p.x, p.y)).unwrap()
    }

    fn apart(placed: &[LayoutBox]) {
        for p in placed {
            for q in placed {
                if p.id < q.id {
                    let apart = p.x + p.width <= q.x || q.x + q.width <= p.x || p.y + p.height <= q.y || q.y + q.height <= p.y;
                    assert!(apart, "{p:?} overlaps {q:?}");
                }
            }
        }
    }

    #[test]
    fn a_box_stands_above_the_boxes_it_links_to() {
        let placed = arrange(&[b(1), b(2), b(3)], &[l(1, 2), l(2, 3)], 0, 0);
        let (y1, y2, y3) = (at(&placed, 1).1, at(&placed, 2).1, at(&placed, 3).1);
        assert!(y1 < y2 && y2 < y3, "{placed:?}");
        // A chain stands in one column.
        assert_eq!((at(&placed, 1).0, at(&placed, 2).0), (at(&placed, 3).0, at(&placed, 3).0));
    }

    #[test]
    fn boxes_never_overlap_and_unlinked_ones_come_last() {
        let boxes: Vec<LayoutBox> = (1..=12).map(b).collect();
        let links = [l(1, 2), l(1, 3), l(4, 3), l(5, 3), l(3, 6)];
        let placed = arrange(&boxes, &links, 20, 0);
        apart(&placed);
        for p in &placed {
            assert_eq!((p.x % 20, p.y % 20), (0, 0), "on the grid: {p:?}");
        }
        let lowest_linked = [1, 2, 3, 4, 5, 6].iter().map(|&i| at(&placed, i).1).max().unwrap();
        assert!((7..=12).all(|i| at(&placed, i).1 > lowest_linked), "{placed:?}");
    }

    #[test]
    fn groups_that_do_not_link_stand_apart() {
        // 1-2 and 3-4 are two groups: side by side, not mixed.
        let placed = arrange(&[b(1), b(2), b(3), b(4)], &[l(1, 2), l(3, 4)], 0, 0);
        apart(&placed);
        let first = at(&placed, 1).0.max(at(&placed, 2).0) + 200;
        let second = at(&placed, 3).0.min(at(&placed, 4).0);
        let other_first = at(&placed, 3).0.max(at(&placed, 4).0) + 200;
        let other_second = at(&placed, 1).0.min(at(&placed, 2).0);
        assert!(first <= second || other_first <= other_second, "{placed:?}");
    }

    #[test]
    fn a_line_that_skips_a_row_bends_around_the_boxes_between() {
        // 1 -> 2 -> 3, and 1 -> 3 skips 2's row: it gets one bend point in that row, beside 2.
        let boxes = [b(1), b(2), b(3)];
        let links = [l(1, 2), l(2, 3), l(1, 3)];
        let placed = arrange(&boxes, &links, 0, 0);
        let routed = route(&boxes, &links, 0, 0);
        assert!(routed[0].via.is_empty() && routed[1].via.is_empty(), "{routed:?}");
        let skip = &routed[2];
        assert_eq!(4, skip.via.len(), "in and out of the row it skips: {routed:?}");
        let (x2, y2) = at(&placed, 2);
        assert_eq!((skip.via[1], skip.via[3]), (y2, y2 + 100), "down through the row's height");
        assert_eq!(skip.via[0], skip.via[2], "straight down");
        assert!(skip.via[0] < x2 || skip.via[0] > x2 + 200, "beside box 2: {skip:?} {placed:?}");
    }

    #[test]
    fn a_box_no_one_links_to_stands_just_above_what_it_links_to() {
        // 1 -> 2 -> 3 -> 4, and 5 links only to 4: 5 stands in 3's row, not at the top.
        let placed = arrange(&[b(1), b(2), b(3), b(4), b(5)], &[l(1, 2), l(2, 3), l(3, 4), l(5, 4)], 0, 0);
        assert_eq!(at(&placed, 3).1, at(&placed, 5).1, "{placed:?}");
    }

    #[test]
    fn a_row_wider_than_the_canvas_wraps_its_groups() {
        let boxes: Vec<LayoutBox> = (1..=8).map(b).collect();
        let links = [l(1, 2), l(3, 4), l(5, 6), l(7, 8)];
        let placed = arrange(&boxes, &links, 0, 1000);
        assert!(placed.iter().all(|p| p.x >= 0 && p.x + p.width <= 1000), "{placed:?}");
        apart(&placed);
    }

    #[test]
    fn a_cycle_is_broken_and_still_placed() {
        let placed = arrange(&[b(1), b(2)], &[l(1, 2), l(2, 1)], 0, 0);
        assert_ne!(at(&placed, 1).1, at(&placed, 2).1);
    }

    #[test]
    fn rows_are_ordered_so_lines_cross_less() {
        // 1 holds 3, 2 holds 4: 3 stands under 1 and 4 under 2, whatever order they came in.
        let placed = arrange(&[b(1), b(2), b(4), b(3)], &[l(1, 3), l(2, 4), l(1, 2)], 0, 0);
        assert_eq!(at(&placed, 1).0 < at(&placed, 2).0, at(&placed, 3).0 < at(&placed, 4).0, "{placed:?}");
    }

    #[test]
    fn a_big_diagram_is_arranged_quickly_and_apart() {
        // About the size of a domain: 60 boxes, 90 links.
        let boxes: Vec<LayoutBox> = (1..=60).map(|i| LayoutBox { id: i, width: 160 + (i % 5) * 30, height: 80 + (i % 7) * 40, x: 0, y: 0 }).collect();
        let links: Vec<LayoutLink> = (1..=90).map(|k| l(1 + (k * 7) % 60, 1 + (k * 13 + 5) % 60)).collect();
        let started = std::time::Instant::now();
        let placed = arrange(&boxes, &links, 20, 4000);
        assert!(started.elapsed().as_secs_f64() < 5.0, "{:?}", started.elapsed());
        apart(&placed);
    }
}
