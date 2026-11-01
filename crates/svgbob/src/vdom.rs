//! A minimal SVG node tree and renderer.
//!
//! This module replaces the `sauron` dependency, which svgbob only ever used
//! to build an element tree and print it. The output is byte-identical to
//! what sauron 0.61 rendered (see `tests/parity.rs`): two-space indentation,
//! a lone text child printed inline, no self-closing tags, attributes of the
//! same name merged in first-occurrence order with their values joined by a
//! single space, and attributes with no values skipped.
//!
//! Text is written as-is: callers escape where escaping is needed, exactly
//! as they did under sauron.

use std::{fmt, marker::PhantomData};

const INDENT: usize = 2;

/// An element attribute: a name and the values collected for it.
///
/// A plain attribute (`x="1"`) carries one value. A class list carries one
/// value per class, and an empty class list carries none, which makes the
/// renderer skip the attribute entirely.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    name: &'static str,
    values: Vec<String>,
}

impl Attribute {
    /// An attribute with a single value.
    pub fn new(name: &'static str, value: impl fmt::Display) -> Self {
        Attribute {
            name,
            values: vec![value.to_string()],
        }
    }

    /// An attribute with any number of values; none means "render nothing".
    pub fn multi(name: &'static str, values: Vec<String>) -> Self {
        Attribute { name, values }
    }
}

/// A node in the tree: an element, text, or a comment.
#[derive(Debug, Clone, PartialEq)]
pub struct Node<MSG> {
    kind: NodeKind<MSG>,
}

#[derive(Debug, Clone, PartialEq)]
enum NodeKind<MSG> {
    Element {
        tag: &'static str,
        attrs: Vec<Attribute>,
        children: Vec<Node<MSG>>,
        _msg: PhantomData<MSG>,
    },
    Text(String),
    Comment(&'static str),
}

/// A text node. Content is written to the output verbatim; the caller
/// escapes it first where needed.
pub fn text<MSG>(content: impl ToString) -> Node<MSG> {
    Node {
        kind: NodeKind::Text(content.to_string()),
    }
}

/// An element node. Adjacent text children get a `<!--separator-->` comment
/// between them, as sauron's `html_element` inserted.
pub fn element<MSG>(
    tag: &'static str,
    attrs: impl IntoIterator<Item = Attribute>,
    children: impl IntoIterator<Item = Node<MSG>>,
) -> Node<MSG> {
    let mut corrected: Vec<Node<MSG>> = vec![];
    for child in children {
        if let Some(last) = corrected.last() {
            if last.is_text() && child.is_text() {
                corrected.push(Node {
                    kind: NodeKind::Comment("separator"),
                });
            }
        }
        corrected.push(child);
    }
    Node {
        kind: NodeKind::Element {
            tag,
            attrs: attrs.into_iter().collect(),
            children: corrected,
            _msg: PhantomData,
        },
    }
}

impl<MSG> Node<MSG> {
    /// Whether this node is a text node.
    fn is_text(&self) -> bool {
        matches!(self.kind, NodeKind::Text(_))
    }

    /// Appends `attrs` to an element node; a non-element is unchanged.
    /// Same-name attributes merge at render time, so appending is also
    /// what sauron's `merge_attributes` amounted to for printed output.
    pub fn with_attributes(
        mut self,
        new_attrs: impl IntoIterator<Item = Attribute>,
    ) -> Self {
        if let NodeKind::Element { ref mut attrs, .. } = self.kind {
            attrs.extend(new_attrs);
        }
        self
    }

    /// Alias of [`Node::with_attributes`], kept for the sauron call shape.
    pub fn merge_attributes(
        self,
        new_attrs: impl IntoIterator<Item = Attribute>,
    ) -> Self {
        self.with_attributes(new_attrs)
    }

    /// Appends children to an element node; a non-element is unchanged.
    pub fn with_children(
        mut self,
        new_children: impl IntoIterator<Item = Node<MSG>>,
    ) -> Self {
        if let NodeKind::Element {
            ref mut children, ..
        } = self.kind
        {
            children.extend(new_children);
        }
        self
    }

    /// Renders with indentation into `buffer`.
    pub fn render(&self, buffer: &mut dyn fmt::Write) -> fmt::Result {
        self.render_with_indent(buffer, 0, false)
    }

    /// Renders without newlines or indentation.
    pub fn render_to_string(&self) -> String {
        let mut buffer = String::new();
        self.render_with_indent(&mut buffer, 0, true)
            .expect("writing to a String cannot fail");
        buffer
    }

    fn render_with_indent(
        &self,
        buffer: &mut dyn fmt::Write,
        indent: usize,
        compressed: bool,
    ) -> fmt::Result {
        match &self.kind {
            NodeKind::Text(content) => write!(buffer, "{content}"),
            NodeKind::Comment(comment) => write!(buffer, "<!--{comment}-->"),
            NodeKind::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                write!(buffer, "<{tag}")?;
                render_attributes(buffer, attrs)?;
                write!(buffer, ">")?;

                let lone_text_child =
                    children.len() == 1 && children[0].is_text();
                if lone_text_child {
                    children[0]
                        .render_with_indent(buffer, indent, compressed)?;
                } else {
                    for child in children {
                        maybe_indent(buffer, indent + 1, compressed)?;
                        child.render_with_indent(
                            buffer,
                            indent + 1,
                            compressed,
                        )?;
                    }
                }
                if !lone_text_child && !children.is_empty() {
                    maybe_indent(buffer, indent, compressed)?;
                }
                write!(buffer, "</{tag}>")
            }
        }
    }
}

fn maybe_indent(
    buffer: &mut dyn fmt::Write,
    indent: usize,
    compressed: bool,
) -> fmt::Result {
    if !compressed {
        write!(buffer, "\n{}", " ".repeat(INDENT * indent))?;
    }
    Ok(())
}

/// Writes the attributes: same-name attributes merge in first-occurrence
/// order, values joined with a space. A merged attribute with no values
/// writes nothing after its separator space -- sauron behaved the same way
/// (an empty class list from `classes([])` was not dropped, it just
/// rendered empty), and the trailing space it leaves on elements such as
/// `<text x=".." y=".." >` is part of the byte-parity contract.
fn render_attributes(
    buffer: &mut dyn fmt::Write,
    attrs: &[Attribute],
) -> fmt::Result {
    let mut merged: Vec<(&'static str, Vec<&str>)> = vec![];
    for attr in attrs {
        let values = attr.values.iter().map(String::as_str);
        if let Some(existing) =
            merged.iter_mut().find(|(name, _)| *name == attr.name)
        {
            existing.1.extend(values);
        } else {
            merged.push((attr.name, values.collect()));
        }
    }
    for (name, values) in merged {
        write!(buffer, " ")?;
        if !values.is_empty() {
            write!(buffer, "{name}=\"{}\"", values.join(" "))?;
        }
    }
    Ok(())
}

/// Element constructors for the tags svgbob draws with.
pub mod elements {
    use super::{element, Attribute, Node};

    macro_rules! declare_elements {
        ($($(#[$doc:meta])* $name:ident => $tag:literal;)*) => {
            $(
                $(#[$doc])*
                pub fn $name<MSG>(
                    attrs: impl IntoIterator<Item = Attribute>,
                    children: impl IntoIterator<Item = Node<MSG>>,
                ) -> Node<MSG> {
                    element($tag, attrs, children)
                }
            )*
        };
    }

    declare_elements! {
        /// The `<svg>` root element.
        svg => "svg";
        /// The `<rect>` element.
        rect => "rect";
        /// The `<circle>` element.
        circle => "circle";
        /// The `<line>` element.
        line => "line";
        /// The `<path>` element.
        path => "path";
        /// The `<polygon>` element.
        polygon => "polygon";
        /// The `<text>` element.
        text => "text";
        /// The `<g>` group element.
        g => "g";
        /// The `<defs>` element.
        defs => "defs";
        /// The `<marker>` element.
        marker => "marker";
        /// The `<style>` element.
        style => "style";
    }
}

/// Attribute constructors for the attributes svgbob sets. Each maps the
/// Rust-friendly name to the SVG spelling (`view_box` to `viewBox`).
pub mod attributes {
    use super::Attribute;
    use std::fmt;

    macro_rules! declare_attributes {
        ($($(#[$doc:meta])* $name:ident => $attr:literal;)*) => {
            $(
                $(#[$doc])*
                pub fn $name(value: impl fmt::Display) -> Attribute {
                    Attribute::new($attr, value)
                }
            )*
        };
    }

    declare_attributes! {
        /// `class`, a single class name.
        class => "class";
        /// `id`.
        id => "id";
        /// `x`.
        x => "x";
        /// `y`.
        y => "y";
        /// `x1`.
        x1 => "x1";
        /// `y1`.
        y1 => "y1";
        /// `x2`.
        x2 => "x2";
        /// `y2`.
        y2 => "y2";
        /// `cx`.
        cx => "cx";
        /// `cy`.
        cy => "cy";
        /// `r`.
        r => "r";
        /// `rx`.
        rx => "rx";
        /// `width`.
        width => "width";
        /// `height`.
        height => "height";
        /// `points`.
        points => "points";
        /// `d`.
        d => "d";
        /// `viewBox`.
        view_box => "viewBox";
        /// `refX`.
        ref_x => "refX";
        /// `refY`.
        ref_y => "refY";
        /// `markerWidth`.
        marker_width => "markerWidth";
        /// `markerHeight`.
        marker_height => "markerHeight";
        /// `orient`.
        orient => "orient";
        /// `xmlns`.
        xmlns => "xmlns";
    }

    /// `class` from a list of names; an empty list renders nothing.
    pub fn classes(
        names: impl IntoIterator<Item = impl ToString>,
    ) -> Attribute {
        Attribute::multi(
            "class",
            names.into_iter().map(|name| name.to_string()).collect(),
        )
    }

    /// `class` from `(name, flag)` pairs, keeping the flagged names.
    pub fn classes_flag<'a>(
        pairs: impl IntoIterator<Item = (&'a str, bool)>,
    ) -> Attribute {
        Attribute::multi(
            "class",
            pairs
                .into_iter()
                .filter(|(_, flag)| *flag)
                .map(|(name, _)| name.to_string())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        attributes::{class, classes, classes_flag, x},
        elements::{circle, rect, style},
        *,
    };

    #[test]
    fn lone_text_child_is_inline() {
        let node: Node<()> = style([], [text("a { b: c; }")]);
        let mut buffer = String::new();
        node.render(&mut buffer).unwrap();
        assert_eq!(buffer, "<style>a { b: c; }</style>");
    }

    #[test]
    fn children_indent_and_close_on_own_line() {
        let node: Node<()> =
            elements::svg([], [rect([x(1)], []), circle([], [])]);
        let mut buffer = String::new();
        node.render(&mut buffer).unwrap();
        assert_eq!(
            buffer,
            "<svg>\n  <rect x=\"1\"></rect>\n  <circle></circle>\n</svg>"
        );
        assert_eq!(
            node.render_to_string(),
            "<svg><rect x=\"1\"></rect><circle></circle></svg>"
        );
    }

    #[test]
    fn same_name_attributes_merge_in_order() {
        let node: Node<()> = rect([class("frame"), class("component")], []);
        assert_eq!(
            node.render_to_string(),
            "<rect class=\"frame component\"></rect>"
        );
    }

    #[test]
    fn empty_class_list_renders_a_lone_space() {
        // sauron wrote the attribute separator space even when the merged
        // attribute had no values; `<text x=".." y=".." >` in the goldens
        // depends on it.
        let node: Node<()> =
            rect([x(1)], []).merge_attributes([classes(Vec::<String>::new())]);
        assert_eq!(node.render_to_string(), "<rect x=\"1\" ></rect>");
        let merged: Node<()> = rect([x(1), class("solid")], [])
            .merge_attributes([classes(Vec::<String>::new())]);
        assert_eq!(
            merged.render_to_string(),
            "<rect x=\"1\" class=\"solid\"></rect>"
        );
    }

    #[test]
    fn adjacent_text_children_get_a_separator() {
        let node: Node<()> = elements::g([], [text("one"), text("two")]);
        assert_eq!(node.render_to_string(), "<g>one<!--separator-->two</g>");
    }

    #[test]
    fn float_values_format_like_std_display() {
        let node: Node<()> =
            rect([x(328.0_f32), super::attributes::y(1.5_f32)], []);
        assert_eq!(
            node.render_to_string(),
            "<rect x=\"328\" y=\"1.5\"></rect>"
        );
    }
}
