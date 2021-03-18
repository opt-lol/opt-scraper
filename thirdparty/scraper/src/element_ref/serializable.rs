use std::io::Error;

use ego_tree::iter::Edge;
use html5ever::serialize::{Serialize, Serializer, TraversalScope};

use crate::{ElementRef, Node};

impl<'a> Serialize for ElementRef<'a> {
    fn serialize<S: Serializer>(
        &self,
        serializer: &mut S,
        traversal_scope: TraversalScope,
    ) -> Result<(), Error> {
        for edge in self.traverse() {
            match edge {
                Edge::Open(node) => {
                    if node == **self && traversal_scope == TraversalScope::ChildrenOnly(None) {
                        continue;
                    }

                    match *node.value() {
                        Node::Doctype(ref doctype) => {
                            serializer.write_doctype(doctype.name())?;
                        }
                        Node::Comment(ref comment) => {
                            serializer.write_comment(comment)?;
                        }
                        Node::Text(ref text) => {
                            serializer.write_text(text)?;
                        }
                        Node::Element(ref elem) => {
                            let mut attrs_vec: Vec<_> =
                                elem.attrs.iter().map(|(k, v)| (k, &v[..])).collect();
                            attrs_vec.sort_by_key(|&(k, _)| k);
                            serializer.start_elem(elem.name.clone(), attrs_vec.into_iter())?;
                        }
                        _ => (),
                    }
                }

                Edge::Close(node) => {
                    if node == **self && traversal_scope == TraversalScope::ChildrenOnly(None) {
                        continue;
                    }

                    if let Some(elem) = node.value().as_element() {
                        serializer.end_elem(elem.name.clone())?;
                    }
                }
            }
        }

        Ok(())
    }
}
