use std::collections::{BTreeSet, VecDeque};

use super::{Edge, EdgeAction, MatchSet, Node};
use crate::parser::{
    ast::{AstKind, GreedyKind, MatchKind, PositionKind, RepeatKind},
    Ast,
};

pub(crate) struct Builder {
    nodes: Vec<Node>,
}

impl Builder {
    pub fn build(ast: &Ast) -> Vec<Node> {
        let mut builder = Builder { nodes: vec![] };
        builder.build_(ast);
        return builder.nodes;
    }

    fn build_(&mut self, ast: &Ast) {
        self.nodes.push(Node { nexts: vec![] }); // root
        self.nodes.push(Node { nexts: vec![] }); // submit
        self.nodes.push(Node { nexts: vec![] }); // fail

        let node_id = self.build_root(ast, 1);

        self.nodes[0].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: node_id,
            is_greedy: true,
        });
    }

    fn build_root(&mut self, ast: &Ast, dst_id: usize) -> usize {
        match &ast.kind {
            AstKind::Group => self.build_group(ast, dst_id),
            AstKind::Union => self.build_union(ast, dst_id),
            AstKind::IncludeSet => self.build_include_set(ast, dst_id),
            AstKind::ExcludeSet => self.build_exclude_set(ast, dst_id),
            AstKind::Star(greedy) => self.build_star(ast, greedy, dst_id),
            AstKind::Plus(greedy) => self.build_plus(ast, greedy, dst_id),
            AstKind::Option(greedy) => self.build_option(ast, greedy, dst_id),
            AstKind::Repeat(n, m, greedy) => self.build_repeat(ast, n, m, greedy, dst_id),
            AstKind::Match(kind) => self.build_match(kind, dst_id),
            AstKind::Position(kind) => self.build_position(kind, dst_id),
        }
    }

    fn build_group(&mut self, ast: &Ast, dst_id: usize) -> usize {
        let mut dst_id = dst_id;
        for child in ast.children.iter().rev() {
            let match_id = self.build_root(child, dst_id);
            dst_id = match_id;
        }
        dst_id
    }

    fn build_union(&mut self, ast: &Ast, dst_id: usize) -> usize {
        let node_id = self.nodes.len();
        self.nodes.push(Node { nexts: vec![] });

        for child in ast.children.iter() {
            let match_id = self.build_root(child, dst_id);
            self.nodes[node_id].nexts.push(Edge {
                action: EdgeAction::Asap,
                next_id: match_id,
                is_greedy: true,
            });
        }
        node_id
    }

    fn build_include_set(&mut self, ast: &Ast, dst_id: usize) -> usize {
        let set_items = Self::build_set_items(ast);

        let node_id = self.nodes.len();
        self.nodes.push(Node {
            nexts: vec![Edge {
                action: EdgeAction::MatchIncludeSet(set_items),
                next_id: dst_id,
                is_greedy: true,
            }],
        });
        node_id
    }

    fn build_exclude_set(&mut self, ast: &Ast, dst_id: usize) -> usize {
        let set_items = Self::build_set_items(ast);

        let node_id = self.nodes.len();
        self.nodes.push(Node {
            nexts: vec![Edge {
                action: EdgeAction::MatchExcludeSet(set_items),
                next_id: dst_id,
                is_greedy: true,
            }],
        });
        node_id
    }

    fn build_set_items(ast: &Ast) -> Vec<MatchSet> {
        let mut set_items = vec![];

        for child in ast.children.iter() {
            match &child.kind {
                AstKind::Match(kind) => match kind {
                    MatchKind::Char(c) => set_items.push(MatchSet::Char(*c)),
                    MatchKind::Range(a, b) => set_items.push(MatchSet::Range(*a, *b)),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }

        set_items
    }

    fn build_star(&mut self, ast: &Ast, greedy: &GreedyKind, dst_id: usize) -> usize {
        let loop_id = self.nodes.len();
        self.nodes.push(Node { nexts: vec![] });

        let match_id = self.build_root(&ast.children[0], loop_id);
        self.nodes[loop_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: match_id,
            is_greedy: true,
        });

        if matches!(*greedy, GreedyKind::Greedy) {
            self.nodes[loop_id].nexts.push(Edge {
                action: EdgeAction::Asap,
                next_id: dst_id,
                is_greedy: true,
            });
        } else {
            self.nodes[loop_id].nexts.insert(
                0,
                Edge {
                    action: EdgeAction::Asap,
                    next_id: dst_id,
                    is_greedy: true,
                },
            );
        }

        if matches!(*greedy, GreedyKind::NonGreedy) {
            self.recursive_set_greedy(loop_id, dst_id, false);
        }

        loop_id
    }

    fn build_plus(&mut self, ast: &Ast, greedy: &GreedyKind, dst_id: usize) -> usize {
        let loop_id = self.nodes.len();
        self.nodes.push(Node { nexts: vec![] });

        let match_id = self.build_root(&ast.children[0], loop_id);
        self.nodes[loop_id].nexts.push(Edge {
            action: EdgeAction::Asap,
            next_id: match_id,
            is_greedy: true,
        });

        if matches!(*greedy, GreedyKind::Greedy) {
            self.nodes[loop_id].nexts.push(Edge {
                action: EdgeAction::Asap,
                next_id: dst_id,
                is_greedy: true,
            });
        } else {
            self.nodes[loop_id].nexts.insert(
                0,
                Edge {
                    action: EdgeAction::Asap,
                    next_id: dst_id,
                    is_greedy: true,
                },
            );
        }

        if matches!(*greedy, GreedyKind::NonGreedy) {
            self.recursive_set_greedy(match_id, dst_id, false);
        }

        match_id
    }

    fn build_option(&mut self, ast: &Ast, greedy: &GreedyKind, dst_id: usize) -> usize {
        let match_id = self.build_root(&ast.children[0], dst_id);

        if matches!(*greedy, GreedyKind::Greedy) {
            self.nodes[match_id].nexts.push(Edge {
                action: EdgeAction::Asap,
                next_id: dst_id,
                is_greedy: true,
            });
        } else {
            self.nodes[match_id].nexts.insert(
                0,
                Edge {
                    action: EdgeAction::Asap,
                    next_id: dst_id,
                    is_greedy: true,
                },
            );
        }

        if matches!(*greedy, GreedyKind::NonGreedy) {
            self.recursive_set_greedy(match_id, dst_id, false);
        }

        match_id
    }

    fn build_repeat(
        &mut self,
        ast: &Ast,
        min: &RepeatKind,
        max: &RepeatKind,
        greedy: &GreedyKind,
        dst_id: usize,
    ) -> usize {
        match (min, max) {
            (RepeatKind::Num(n), RepeatKind::Num(m)) if n == m => {
                self.build_repeat_count(ast, *n, dst_id)
            }
            (RepeatKind::Num(n), RepeatKind::Num(m)) => {
                self.build_repeat_range(ast, *n, *m, greedy, dst_id)
            }
            (RepeatKind::Num(c), RepeatKind::Infinity) => {
                self.build_repeat_min(ast, *c, greedy, dst_id)
            }
            (RepeatKind::Infinity, _) => {
                unreachable!()
            }
        }
    }

    fn build_repeat_count(&mut self, ast: &Ast, count: u32, dst_id: usize) -> usize {
        let mut dst_id = dst_id;

        let child = &ast.children[0];
        for _ in 0..count {
            let match_id = self.build_root(child, dst_id);
            dst_id = match_id;
        }

        dst_id
    }

    fn build_repeat_min(
        &mut self,
        ast: &Ast,
        count: u32,
        greedy: &GreedyKind,
        dst_id: usize,
    ) -> usize {
        let loop_id = self.build_star(ast, greedy, dst_id);
        self.build_repeat_count(ast, count, loop_id)
    }

    fn build_repeat_range(
        &mut self,
        ast: &Ast,
        min: u32,
        max: u32,
        greedy: &GreedyKind,
        dst_id: usize,
    ) -> usize {
        let mut match_id = dst_id;

        let child = &ast.children[0];
        for _ in min..max {
            let repeat_id = self.build_root(child, match_id);
            if matches!(*greedy, GreedyKind::Greedy) {
                self.nodes[repeat_id].nexts.push(Edge {
                    action: EdgeAction::Asap,
                    next_id: dst_id,
                    is_greedy: true,
                });
            } else {
                self.nodes[repeat_id].nexts.insert(
                    0,
                    Edge {
                        action: EdgeAction::Asap,
                        next_id: dst_id,
                        is_greedy: true,
                    },
                );
            }
            match_id = repeat_id;
        }

        if matches!(*greedy, GreedyKind::NonGreedy) {
            self.recursive_set_greedy(match_id, dst_id, false);
        }

        self.build_repeat_count(ast, min, match_id)
    }

    fn build_match(&mut self, kind: &MatchKind, dst_id: usize) -> usize {
        let node_id = self.nodes.len();

        let action = match kind {
            MatchKind::Any => EdgeAction::MatchAny,
            MatchKind::Char(c) => EdgeAction::Match(*c),
            MatchKind::Range(_, _) => unreachable!(),
        };

        self.nodes.push(Node {
            nexts: vec![Edge {
                action,
                next_id: dst_id,
                is_greedy: true,
            }],
        });

        node_id
    }

    fn build_position(&mut self, position: &PositionKind, dst_id: usize) -> usize {
        let node_id = self.nodes.len();

        let action = match position {
            PositionKind::SoL => EdgeAction::MatchSOL,
            PositionKind::EoL => EdgeAction::MatchEOL,
        };

        self.nodes.push(Node {
            nexts: vec![Edge {
                action,
                next_id: dst_id,
                is_greedy: true,
            }],
        });

        node_id
    }

    fn recursive_set_greedy(&mut self, start_id: usize, end_id: usize, is_greedy: bool) {
        let mut finished = BTreeSet::new();
        finished.insert(end_id);

        let mut q = VecDeque::new();
        q.push_back(start_id);

        while let Some(id) = q.pop_front() {
            if !finished.insert(id) {
                continue;
            }

            for edge in self.nodes[id].nexts.iter_mut() {
                edge.is_greedy = is_greedy;
                q.push_back(edge.next_id);
            }
        }
    }
}
