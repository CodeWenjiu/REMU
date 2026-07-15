//! Dinic's maxflow algorithm on a random bipartite graph.
//! Size: 10 (test). Checksum: 0x0000019c.

use crate::bench::Bench;
use alloc::vec::Vec;
use core::fmt::Write;

const SIZE: usize = 10;
const CHECKSUM: u32 = 0x0000019c;
const INF: i32 = 0x3f3f3f;

struct Edge {
    to: usize,
    cap: i32,
    flow: i32,
}

pub(crate) struct Dinic;

impl Bench for Dinic {
    fn run<W: Write>(_w: &mut W) -> bool {
        let n = SIZE;
        let s = 2 * n;
        let t = 2 * n + 1;
        let total_nodes = 2 * n + 2;

        let nold = (total_nodes - 2) / 2;
        let maxm = (nold * nold + nold * 2) * 2;

        let mut edges: Vec<Edge> = Vec::with_capacity(maxm);
        let mut head: Vec<i32> = alloc::vec![-1i32; total_nodes];
        let mut nxt: Vec<i32> = Vec::with_capacity(maxm);
        let mut vis: Vec<bool> = alloc::vec![false; total_nodes];
        let mut d: Vec<i32> = alloc::vec![0i32; total_nodes];
        let mut queue: Vec<usize> = alloc::vec![0usize; total_nodes];

        unsafe {
            edges.set_len(maxm);
            nxt.set_len(maxm);
        }

        let mut m = 0usize;

        let mut add_edge = |u: usize, v: usize, c: i32| {
            if c == 0 {
                return;
            }
            edges[m] = Edge {
                to: v,
                cap: c,
                flow: 0,
            };
            nxt[m] = head[u];
            head[u] = m as i32;
            m += 1;
            edges[m] = Edge {
                to: u,
                cap: 0,
                flow: 0,
            };
            nxt[m] = head[v];
            head[v] = m as i32;
            m += 1;
        };

        let mut seed = 1u32;
        for i in 0..n {
            for j in 0..n {
                add_edge(i, n + j, (rand15(&mut seed) % 10) as i32);
            }
        }
        for i in 0..n {
            add_edge(s, i, (rand15(&mut seed) % 1000) as i32);
            add_edge(n + i, t, (rand15(&mut seed) % 1000) as i32);
        }

        // Maxflow
        let mut flow = 0i32;
        while bfs(s, t, &head, &nxt, &edges, &mut vis, &mut d, &mut queue) {
            let mut cur_local = head.clone();
            flow = flow.wrapping_add(dfs(s, t, INF, &head, &nxt, &mut edges, &d, &mut cur_local));
        }

        flow as u32 == CHECKSUM
    }
}

fn bfs(
    s: usize,
    t: usize,
    head: &[i32],
    nxt: &[i32],
    edges: &[Edge],
    vis: &mut [bool],
    d: &mut [i32],
    queue: &mut [usize],
) -> bool {
    for v in vis.iter_mut() {
        *v = false;
    }
    let mut qf = 0usize;
    let mut qr = 0usize;
    queue[qr] = s;
    qr += 1;
    d[s] = 0;
    vis[s] = true;
    while qf != qr {
        let x = queue[qf];
        qf += 1;
        let mut i = head[x];
        while i != -1 {
            let e = &edges[i as usize];
            if !vis[e.to] && e.cap > e.flow {
                vis[e.to] = true;
                d[e.to] = d[x] + 1;
                queue[qr] = e.to;
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
    edges: &mut [Edge],
    d: &[i32],
    cur: &mut [i32],
) -> i32 {
    if x == t || a == 0 {
        return a;
    }
    let mut flow = 0i32;
    let mut remaining = a;
    let mut i = cur[x];
    while i != -1 {
        let idx = i as usize;
        if d[x] + 1 == d[edges[idx].to] {
            let cap_flow = edges[idx].cap - edges[idx].flow;
            let f = dfs(
                edges[idx].to,
                t,
                if remaining < cap_flow {
                    remaining
                } else {
                    cap_flow
                },
                head,
                nxt,
                edges,
                d,
                cur,
            );
            if f > 0 {
                edges[idx].flow += f;
                edges[idx ^ 1].flow -= f;
                flow += f;
                remaining -= f;
                if remaining == 0 {
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
