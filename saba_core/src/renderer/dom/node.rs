use crate::renderer::html::attribute::Attribute;
use alloc::format;
use alloc::rc::Rc;
use alloc::rc::Weak;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::str::FromStr;

type WeakRefCell<T> = Weak<RefCell<T>>;
type RcRefCell<T> = Rc<RefCell<T>>;

#[derive(Debug, Clone)]
pub struct Node {
    pub kind: NodeKind,
    // window object
    window: WeakRefCell<Window>,
    // parent node
    parent: WeakRefCell<Node>,
    first_child: Option<RcRefCell<Node>>,
    last_child: WeakRefCell<Node>,
    previous_sibling: WeakRefCell<Node>,
    next_sibling: Option<RcRefCell<Node>>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl Node {
    pub fn new(kind: NodeKind) -> Self {
        Self {
            kind,
            window: Weak::new(),
            parent: Weak::new(),
            first_child: None,
            last_child: Weak::new(),
            previous_sibling: Weak::new(),
            next_sibling: None,
        }
    }

    pub fn kind(&self) -> NodeKind {
        self.kind.clone()
    }

    pub fn set_window(&mut self, window: WeakRefCell<Window>) {
        self.window = window;
    }

    pub fn set_parent(&mut self, parent: WeakRefCell<Node>) {
        self.parent = parent;
    }

    pub fn parent(&self) -> WeakRefCell<Node> {
        self.parent.clone()
    }

    pub fn set_first_child(&mut self, first_child: Option<RcRefCell<Node>>) {
        self.first_child = first_child;
    }

    pub fn first_child(&self) -> Option<RcRefCell<Node>> {
        self.first_child.as_ref().cloned()
    }

    pub fn set_last_child(&mut self, last_child: WeakRefCell<Node>) {
        self.last_child = last_child;
    }

    pub fn last_child(&self) -> WeakRefCell<Node> {
        self.last_child.clone()
    }

    pub fn set_previous_sibling(&mut self, sibling: WeakRefCell<Node>) {
        self.previous_sibling = sibling;
    }

    pub fn previous_sibling(&self) -> WeakRefCell<Node> {
        self.previous_sibling.clone()
    }

    pub fn set_next_sibling(&mut self, sibling: Option<RcRefCell<Node>>) {
        self.next_sibling = sibling;
    }

    pub fn next_sibling(&self) -> Option<RcRefCell<Node>> {
        self.next_sibling.as_ref().cloned()
    }

    pub fn get_element(&self) -> Option<Element> {
        match self.kind {
            NodeKind::Document | NodeKind::Text(_) => None,
            NodeKind::Element(ref e) => Some(e.clone()),
        }
    }

    pub fn element_kind(&self) -> Option<ElementKind> {
        match self.kind {
            NodeKind::Document | NodeKind::Text(_) => None,
            NodeKind::Element(ref e) => Some(e.kind()),
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub enum NodeKind {
    Document,
    Element(Element),
    Text(String),
}

impl PartialEq for NodeKind {
    fn eq(&self, other: &Self) -> bool {
        match &self {
            NodeKind::Document => matches!(other, NodeKind::Document),
            NodeKind::Element(e1) => match &other {
                NodeKind::Element(e2) => e1.kind == e2.kind,
                _ => false,
            },
            NodeKind::Text(_) => matches!(other, NodeKind::Text(_)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    document: RcRefCell<Node>,
}

impl Window {
    pub fn new() -> Self {
        let document = Rc::new(RefCell::new(Node::new(NodeKind::Document)));
        let window = Self { document };

        window
            .document
            .borrow_mut()
            .set_window(Rc::downgrade(&Rc::new(RefCell::new(window.clone()))));

        window
    }

    pub fn document(&self) -> RcRefCell<Node> {
        self.document.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    kind: ElementKind,
    attributes: Vec<Attribute>,
}

impl Element {
    pub fn new(element_name: &str, attributes: Vec<Attribute>) -> Self {
        Self {
            kind: ElementKind::from_str(element_name)
                .expect("Failed to convert string to ElementKind"),
            attributes,
        }
    }

    pub fn kind(&self) -> ElementKind {
        self.kind
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ElementKind {
    Html,
    Head,
    Style,
    Script,
    Body,
    P,
    H1,
    H2,
    A,
}

impl FromStr for ElementKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "html" => Ok(ElementKind::Html),
            "head" => Ok(ElementKind::Head),
            "style" => Ok(ElementKind::Style),
            "script" => Ok(ElementKind::Script),
            "body" => Ok(ElementKind::Body),
            "p" => Ok(ElementKind::P),
            "h1" => Ok(ElementKind::H1),
            "h2" => Ok(ElementKind::H2),
            "a" => Ok(ElementKind::A),
            _ => Err(format!("Unimplemented element name {:?}", s)),
        }
    }
}
