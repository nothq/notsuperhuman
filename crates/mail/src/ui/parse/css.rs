use super::dom::html_attr;
use super::style_values::apply_style_declaration_with_inherited;
use super::MAIL_CSS_VIEWPORT_WIDTH;
use crate::ui::types::*;
use html5ever::Attribute;
use markup5ever_rcdom::{Handle, NodeData};
use std::cell::RefCell;

mod parser;

use parser::{collect_css_rules, parse_css_declarations, without_css_comments};

#[derive(Default)]
pub(super) struct MailCssRules {
    rules: Vec<MailCssRule>,
}

pub(super) struct MailCssCascadeContext<'a> {
    node: &'a Handle,
    tag_name: &'a str,
    attrs: &'a RefCell<Vec<Attribute>>,
    inherited: &'a MailStyle,
}

impl<'a> MailCssCascadeContext<'a> {
    pub(super) fn new(
        node: &'a Handle,
        tag_name: &'a str,
        attrs: &'a RefCell<Vec<Attribute>>,
        inherited: &'a MailStyle,
    ) -> Self {
        Self {
            node,
            tag_name,
            attrs,
            inherited,
        }
    }
}

impl MailCssRules {
    pub(super) fn from_style_texts(style_texts: Vec<String>) -> Self {
        let mut rules = Vec::new();
        for style in style_texts {
            collect_css_rules(&without_css_comments(&style), &mut rules);
        }
        Self { rules }
    }

    pub(super) fn apply(&self, context: MailCssCascadeContext<'_>, style: &mut MailStyle) {
        let element = MailCssElement::new(context.tag_name, context.attrs);
        let ancestors = element_ancestors(context.node);
        let previous_sibling = previous_element_sibling(context.node);
        let matching_rules = self.matching_rules(&element, &ancestors, previous_sibling.as_ref());
        let inline_declarations = html_attr(context.attrs, "style")
            .as_deref()
            .map(parse_css_declarations)
            .unwrap_or_default();
        apply_declaration_pass(
            &matching_rules,
            &inline_declarations,
            context.inherited,
            style,
            MailCssDeclarationPass::FontSize,
        );
        apply_declaration_pass(
            &matching_rules,
            &inline_declarations,
            context.inherited,
            style,
            MailCssDeclarationPass::Remaining,
        );
    }

    fn matching_rules(
        &self,
        element: &MailCssElement,
        ancestors: &[MailCssElement],
        previous_sibling: Option<&MailCssElement>,
    ) -> Vec<(usize, &MailCssRule)> {
        let mut matching_rules = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.selector.matches(element, ancestors, previous_sibling))
            .collect::<Vec<_>>();
        matching_rules
            .sort_by_key(|(source_order, rule)| (rule.selector.specificity(), *source_order));
        matching_rules
    }
}

#[derive(Clone, Copy)]
enum MailCssDeclarationPass {
    FontSize,
    Remaining,
}

impl MailCssDeclarationPass {
    fn includes(self, declaration: &MailCssDeclaration) -> bool {
        let is_font_size = declaration.property.eq_ignore_ascii_case("font-size");
        match self {
            Self::FontSize => is_font_size,
            Self::Remaining => !is_font_size,
        }
    }
}

fn apply_declaration_pass(
    matching_rules: &[(usize, &MailCssRule)],
    inline_declarations: &[MailCssDeclaration],
    inherited: &MailStyle,
    style: &mut MailStyle,
    pass: MailCssDeclarationPass,
) {
    for important in [false, true] {
        let rule_declarations = matching_rules
            .iter()
            .flat_map(|(_, rule)| rule.declarations.iter());
        for declaration in rule_declarations.chain(inline_declarations) {
            if declaration.important == important && pass.includes(declaration) {
                apply_style_declaration_with_inherited(
                    &declaration.property,
                    &declaration.value,
                    inherited,
                    style,
                );
            }
        }
    }
}

struct MailCssRule {
    selector: MailCssSelector,
    declarations: Vec<MailCssDeclaration>,
}

#[derive(Clone)]
struct MailCssDeclaration {
    property: String,
    value: String,
    important: bool,
}

enum MailCssSelector {
    Compound(MailCssCompoundSelector),
    Descendant {
        ancestors: Vec<MailCssCompoundSelector>,
        target: MailCssCompoundSelector,
    },
    Child {
        ancestors: Vec<MailCssCompoundSelector>,
        target: MailCssCompoundSelector,
    },
    Adjacent {
        previous: MailCssCompoundSelector,
        target: MailCssCompoundSelector,
    },
}

struct MailCssElement {
    tag: String,
    id: Option<String>,
    classes: Vec<String>,
    /// Every attribute, name lowercased, for attribute selectors.
    attributes: Vec<(String, String)>,
}

impl MailCssElement {
    fn new(tag: &str, attrs: &RefCell<Vec<Attribute>>) -> Self {
        Self {
            tag: tag.to_ascii_lowercase(),
            id: html_attr(attrs, "id"),
            classes: element_classes(attrs),
            attributes: attrs
                .borrow()
                .iter()
                .map(|attr| {
                    (
                        attr.name.local.as_ref().to_ascii_lowercase(),
                        attr.value.to_string(),
                    )
                })
                .collect(),
        }
    }
}

#[derive(Clone)]
struct MailCssCompoundSelector {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attributes: Vec<MailCssAttributeSelector>,
}

/// `[name]`, `[name=value]` or `[name~=value]`. Email stylesheets use the
/// exact form to aim rules at clients that keep the attribute, such as
/// `table[class="background_main"] .social_icon_margin`.
#[derive(Clone)]
struct MailCssAttributeSelector {
    name: String,
    value: Option<(MailCssAttributeMatch, String)>,
}

#[derive(Clone, Copy)]
enum MailCssAttributeMatch {
    Exact,
    Word,
}

impl MailCssAttributeSelector {
    fn matches(&self, attributes: &[(String, String)]) -> bool {
        attributes.iter().any(|(name, actual)| {
            *name == self.name
                && match &self.value {
                    None => true,
                    Some((MailCssAttributeMatch::Exact, value)) => actual == value,
                    Some((MailCssAttributeMatch::Word, value)) => {
                        actual.split_ascii_whitespace().any(|word| word == value)
                    }
                }
        })
    }
}

impl MailCssCompoundSelector {
    fn specificity(&self) -> (usize, usize, usize) {
        (
            usize::from(self.id.is_some()),
            self.classes.len() + self.attributes.len(),
            usize::from(self.tag.is_some()),
        )
    }

    fn matches(&self, element: &MailCssElement) -> bool {
        self.tag
            .as_deref()
            .is_none_or(|required| required == element.tag)
            && self
                .id
                .as_deref()
                .is_none_or(|required| Some(required) == element.id.as_deref())
            && classes_match(&self.classes, &element.classes)
            && self
                .attributes
                .iter()
                .all(|attribute| attribute.matches(&element.attributes))
    }
}

impl MailCssSelector {
    fn specificity(&self) -> (usize, usize, usize) {
        match self {
            Self::Compound(selector) => selector.specificity(),
            Self::Descendant { ancestors, target } | Self::Child { ancestors, target } => ancestors
                .iter()
                .chain(std::iter::once(target))
                .map(MailCssCompoundSelector::specificity)
                .fold((0, 0, 0), add_specificity),
            Self::Adjacent { previous, target } => {
                add_specificity(previous.specificity(), target.specificity())
            }
        }
    }

    fn matches(
        &self,
        element: &MailCssElement,
        ancestors: &[MailCssElement],
        previous_sibling: Option<&MailCssElement>,
    ) -> bool {
        match self {
            Self::Compound(selector) => selector.matches(element),
            Self::Descendant {
                ancestors: required,
                target,
            } => target.matches(element) && descendant_elements_match(required, ancestors),
            Self::Child {
                ancestors: required,
                target,
            } => {
                target.matches(element)
                    && ancestors
                        .get(ancestors.len().saturating_sub(required.len())..)
                        .is_some_and(|actual| {
                            actual.len() == required.len()
                                && required
                                    .iter()
                                    .zip(actual)
                                    .all(|(selector, element)| selector.matches(element))
                        })
            }
            Self::Adjacent { previous, target } => {
                target.matches(element)
                    && previous_sibling.is_some_and(|element| previous.matches(element))
            }
        }
    }
}

fn add_specificity(
    left: (usize, usize, usize),
    right: (usize, usize, usize),
) -> (usize, usize, usize) {
    (left.0 + right.0, left.1 + right.1, left.2 + right.2)
}

fn classes_match(required: &[String], classes: &[String]) -> bool {
    required
        .iter()
        .all(|required| classes.iter().any(|class| class == required))
}

fn descendant_elements_match(
    required: &[MailCssCompoundSelector],
    ancestors: &[MailCssElement],
) -> bool {
    let mut search_start = 0;
    for selector in required {
        let Some(offset) = ancestors[search_start..]
            .iter()
            .position(|element| selector.matches(element))
        else {
            return false;
        };
        search_start += offset + 1;
    }
    true
}

fn element_ancestors(node: &Handle) -> Vec<MailCssElement> {
    let mut ancestors = Vec::new();
    let mut current = parent_handle(node);
    while let Some(parent) = current {
        if let NodeData::Element { attrs, name, .. } = &parent.data {
            ancestors.push(MailCssElement::new(name.local.as_ref(), attrs));
        }
        current = parent_handle(&parent);
    }
    ancestors.reverse();
    ancestors
}

fn parent_handle(node: &Handle) -> Option<Handle> {
    let parent = node.parent.take();
    let handle = parent.as_ref().and_then(std::rc::Weak::upgrade);
    node.parent.set(parent);
    handle
}

fn previous_element_sibling(node: &Handle) -> Option<MailCssElement> {
    let parent = parent_handle(node)?;
    let children = parent.children.borrow();
    let index = children
        .iter()
        .position(|child| std::rc::Rc::ptr_eq(child, node))?;
    children[..index].iter().rev().find_map(|child| {
        let NodeData::Element { attrs, name, .. } = &child.data else {
            return None;
        };
        Some(MailCssElement::new(name.local.as_ref(), attrs))
    })
}

pub(super) fn element_classes(attrs: &RefCell<Vec<Attribute>>) -> Vec<String> {
    html_attr(attrs, "class")
        .unwrap_or_default()
        .split_whitespace()
        .map(|class| class.to_ascii_lowercase())
        .collect()
}
