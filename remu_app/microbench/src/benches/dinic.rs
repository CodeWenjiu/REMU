//! Dinic's maxflow on a random bipartite graph.

use crate::bench::{Bench, Size};
use alloc::vec::Vec;
use core::fmt::Write;

pub(crate) struct Dinic;

impl Bench for Dinic {
    fn ref_time_usec(size: Size) -> u64 {
        match size {
            Size::Ref => 1267,
            Size::Huge => 5081,
            _ => 0,
        }
    }
    fn run<W: Write>(_w: &mut W, size: Size) -> bool {
        let (n, checksum) = match size {
            Size::Test => (10, 0x0000019c),
            Size::Train => (80, 0x00004f99),
            Size::Ref => (128, 0x0000c248),
            Size::Huge => (190, 0x00014695),
        };
        let s = 2 * n;
        let t = 2 * n + 1;
        let total = 2 * n + 2;
        let nold = (total - 2) / 2;
        let maxm = (nold * nold + nold * 2) * 2;

        let mut edges: Vec<(usize, i32, i32)> = Vec::with_capacity(maxm);
        let mut head: Vec<i32> = alloc::vec![-1i32; total];
        let mut nxt: Vec<i32> = Vec::with_capacity(maxm);
        let mut vis: Vec<bool> = alloc::vec![false; total];
        let mut d: Vec<i32> = alloc::vec![0i32; total];
        let mut queue: Vec<usize> = alloc::vec![0usize; total];
        unsafe {
            edges.set_len(maxm);
            nxt.set_len(maxm);
        }

        let mut m = 0usize;
        let mut add_edge = |u: usize, v: usize, c: i32| {
            if c == 0 {
                return;
            }
            edges[m] = (v, c, 0);
            nxt[m] = head[u];
            head[u] = m as i32;
            m += 1;
            edges[m] = (u, 0, 0);
            nxt[m] = head[v];
            head[v] = m as i32;
            m += 1;
        };

        let mut seed = 1u32;
        for i in 0..n {
            for _j in 0..n {
                add_edge(i, n + _j, (rand15(&mut seed) % 10) as i32);
            }
        }
        for i in 0..n {
            add_edge(s, i, (rand15(&mut seed) % 1000) as i32);
            add_edge(n + i, t, (rand15(&mut seed) % 1000) as i32);
        }

        let mut flow = 0i32;
        while bfs(s, t, &head, &nxt, &edges, &mut vis, &mut d, &mut queue) {
            let mut cur = head.clone();
            flow = flow.wrapping_add(dfs(s, t, 0x3f3f3f, &head, &nxt, &mut edges, &d, &mut cur));
        }
        flow as u32 == checksum
    }
}

fn bfs(
    s: usize,
    t: usize,
    head: &[i32],
    nxt: &[i32],
    edges: &[(usize, i32, i32)],
    vis: &mut [bool],
    d: &mut [i32],
    queue: &mut [usize],
) -> bool {
    for v in vis.iter_mut() {
        *v = false;
    }
    let mut qf = 0;
    let mut qr = 0;
    queue[qr] = s;
    qr += 1;
    d[s] = 0;
    vis[s] = true;
    while qf != qr {
        let x = queue[qf];
        qf += 1;
        let mut i = head[x];
        while i != -1 {
            let (to, cap, flow) = edges[i as usize];
            if !vis[to] && cap > flow {
                vis[to] = true;
                d[to] = d[x] + 1;
                queue[qr] = to;
                qr += 1;
            }
            i = nxt[i as usize];
        }
    }
    vis[t]
}

fn dfs(
    x: usize,
    t: usize,
    a: i32,
    head: &[i32],
    nxt: &[i32],
    edges: &mut [(usize, i32, i32)],
    d: &[i32],
    cur: &mut [i32],
) -> i32 {
    if x == t || a == 0 {
        return a;
    }
    let mut flow = 0i32;
    let mut rem = a;
    let mut i = cur[x];
    while i != -1 {
        let idx = i as usize;
        if d[x] + 1 == d[edges[idx].0] {
            let cap = edges[idx].1 - edges[idx].2;
            let f = dfs(
                edges[idx].0,
                t,
                if rem < cap { rem } else { cap },
                head,
                nxt,
                edges,
                d,
                cur,
            );
            if f > 0 {
                edges[idx].2 += f;
                edges[idx ^ 1].2 -= f;
                flow += f;
                rem -= f;
                if rem == 0 {
                    break;
                }
            }
        }
        i = nxt[idx];
        cur[x] = i;
    }
    flow
}

fn rand15(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    (*seed >> 16) & 0x7fff
}
