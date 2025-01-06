use std::fmt;
use std::fmt::Debug;
use polonius_the_crab::{exit_polonius, polonius, polonius_break, polonius_return};
use serde::{Deserialize, Serialize};

pub const DIRECTORY_TREE_DB_KEY: &str = "directory_tree";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub items: u32,
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(name: String) -> Node {
        Node {
            name,
            items: 0,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: Node) {
        self.children.push(child);
    }

    pub fn pretty_print(&self, f: &mut fmt::Formatter<'_>, prefix: &str, is_last: bool) -> fmt::Result {
        writeln!(f, "{}{}─ {} ({} items)", prefix, if is_last { "└" } else { "├" }, self.name, self.items)?;
        let new_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });
        for (i, child) in self.children.iter().enumerate() {
            child.pretty_print(f, &new_prefix, i == self.children.len() - 1)?;
        }
        Ok(())
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct DirectoryTree {
    root: Node,
}

impl DirectoryTree {
    pub fn new() -> DirectoryTree {
        DirectoryTree {
            root: Node::new(".".to_string()),
        }
    }

    pub fn add_path(&mut self, path: &str) {
        let mut current_node = &mut self.root;
        for part in path.split('/') {
            if part.is_empty() {
                continue;
            }

            current_node = get_or_insert(current_node, part);
        }

        current_node.items += 1;
    }
}

impl Debug for DirectoryTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.root.pretty_print(f, "", true)
    }
}

fn get_or_insert<'a>(mut node: &'a mut Node, part: &str) -> &'a mut Node {
    polonius!(|node| -> &'polonius mut Node  {
        if let Some(v) = node.children.iter_mut().find(|ch| ch.name == part) {
            polonius_return!(v);
        }
    });
    let new_node = Node::new(part.to_string());
    node.add_child(new_node);
    node.children.last_mut().unwrap()
}


