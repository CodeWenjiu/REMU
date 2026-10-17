//! XML reading: roxmltree documents flattened into an owned element tree with
//! MJCF `<include>` resolution.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct Element {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
}

impl Element {
    pub(crate) fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub(crate) fn children_named<'a>(&'a self, tag: &'a str) -> impl Iterator<Item = &'a Element> {
        self.children.iter().filter(move |c| c.tag == tag)
    }
}

/// A parsed document plus every file that was read to produce it (includes
/// resolved), for build-script change tracking.
pub(crate) struct Loaded {
    pub root: Element,
    pub files: Vec<PathBuf>,
}

/// Parse `path`, splicing `<include file="...">` children in place of the
/// include element (resolved relative to the including file's directory).
pub(crate) fn load_file(path: &Path) -> Result<Loaded, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|e| format!("cannot parse {}: {e}", path.display()))?;
    let root_el = doc.root_element();
    if root_el.tag_name().name() != "mujoco" {
        return Err(format!(
            "{}: root element is <{}>, expected <mujoco>",
            path.display(),
            root_el.tag_name().name()
        ));
    }
    let mut files = vec![path.to_path_buf()];
    let children = convert_children(root_el, path.parent(), &mut files)?;
    Ok(Loaded {
        root: Element {
            tag: root_el.tag_name().name().to_owned(),
            attrs: collect_attrs(root_el),
            children,
        },
        files,
    })
}

/// Parse an XML string (test fixtures; `<include>` is not available).
#[cfg(test)]
pub(crate) fn load_str(xml: &str) -> Result<Element, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("cannot parse fixture: {e}"))?;
    let root_el = doc.root_element();
    let mut files = Vec::new();
    Ok(Element {
        tag: root_el.tag_name().name().to_owned(),
        attrs: collect_attrs(root_el),
        children: convert_children(root_el, None, &mut files)?,
    })
}

fn collect_attrs(node: roxmltree::Node) -> Vec<(String, String)> {
    node.attributes()
        .map(|a| (a.name().to_owned(), a.value().to_owned()))
        .collect()
}

fn convert_children(
    node: roxmltree::Node,
    base_dir: Option<&Path>,
    files: &mut Vec<PathBuf>,
) -> Result<Vec<Element>, String> {
    let mut out = Vec::new();
    for child in node.children().filter(|c| c.is_element()) {
        if child.tag_name().name() == "include" {
            let file = child
                .attribute("file")
                .ok_or_else(|| "<include> without a `file` attribute".to_owned())?;
            let base = base_dir
                .ok_or_else(|| format!("<include file={file:?}> requires a file context"))?;
            let path = base.join(file);
            let text = fs::read_to_string(&path)
                .map_err(|e| format!("cannot read included {}: {e}", path.display()))?;
            let doc = roxmltree::Document::parse(&text)
                .map_err(|e| format!("cannot parse included {}: {e}", path.display()))?;
            files.push(path.clone());
            out.extend(convert_children(doc.root_element(), path.parent(), files)?);
            continue;
        }
        out.push(convert_element(child, base_dir, files)?);
    }
    Ok(out)
}

fn convert_element(
    node: roxmltree::Node,
    base_dir: Option<&Path>,
    files: &mut Vec<PathBuf>,
) -> Result<Element, String> {
    Ok(Element {
        tag: node.tag_name().name().to_owned(),
        attrs: collect_attrs(node),
        children: convert_children(node, base_dir, files)?,
    })
}
