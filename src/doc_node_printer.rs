// Copyright 2020-2022 the Deno authors. All rights reserved. MIT license.

use crate::colors;
use crate::display::display_abstract;
use crate::display::display_async;
use crate::display::display_generator;
use crate::display::Indent;
use crate::display::SliceDisplayer;
use crate::js_doc::JsDoc;
use crate::js_doc::JsDocTag;
use crate::node::DocNode;
use crate::node::DocNodeKind;

use std::fmt;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::fmt::Write;

pub fn print_doc_node<'a>(
  node: &'a DocNode,
  private: bool,
  has_overloads: bool,
) -> String {
  let mut str = String::new();

  let printer = DocNodePrinter {
    doc_node: node,
    private,
    has_overloads,
    indent: 0,
    use_color: false,
  };

  write!(str, "{}", printer).unwrap();
  str
}

pub fn print_doc_nodes_assoc<'a>(
  nodes: &'a [DocNode],
  private: bool,
) -> Vec<(DocNode, String)> {
  let mut sorted = Vec::from(nodes);

  sorted.sort_unstable_by(|a, b| {
    let kind_cmp = kind_order(&a.kind).cmp(&kind_order(&b.kind));
    if kind_cmp == core::cmp::Ordering::Equal {
      a.name.cmp(&b.name)
    } else {
      kind_cmp
    }
  });

  sorted
    .clone()
    .into_iter()
    .map(|node| {
      let has_overloads = if node.kind == DocNodeKind::Function {
        sorted
          .iter()
          .filter(|n| n.kind == DocNodeKind::Function && n.name == node.name)
          .count()
          > 1
      } else {
        false
      };

      (node.clone(), print_doc_node(&node, private, has_overloads))
    })
    .collect()
}

fn kind_order(kind: &DocNodeKind) -> i64 {
  match kind {
    DocNodeKind::ModuleDoc => 0,
    DocNodeKind::Function => 1,
    DocNodeKind::Variable => 2,
    DocNodeKind::Class => 3,
    DocNodeKind::Enum => 4,
    DocNodeKind::Interface => 5,
    DocNodeKind::TypeAlias => 6,
    DocNodeKind::Namespace => 7,
    DocNodeKind::Import => 8,
  }
}

pub struct DocNodePrinter<'a> {
  doc_node: &'a DocNode,
  private: bool,
  has_overloads: bool,
  indent: i64,
  use_color: bool,
}

impl<'a> fmt::Display for DocNodePrinter<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    self.format(
      f,
      self.doc_node,
      self.indent,
      self.has_overloads,
      self.use_color,
    )
  }
}

impl<'a> DocNodePrinter<'a> {
  fn format(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
    indent: i64,
    has_overloads: bool,
    use_color: bool,
  ) -> FmtResult {
    if use_color {
      colors::enable_color();
    }

    self.format_signature(w, node, indent, has_overloads)?;
    self.format_jsdoc(w, &node.js_doc, indent + 1)?;

    writeln!(w)?;

    match node.kind {
      DocNodeKind::Class => self.format_class(w, node)?,
      DocNodeKind::Enum => self.format_enum(w, node)?,
      DocNodeKind::Interface => self.format_interface(w, node)?,
      DocNodeKind::Namespace => self.format_namespace(w, node)?,
      _ => {}
    }

    Ok(())
  }

  fn format_signature(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
    indent: i64,
    has_overloads: bool,
  ) -> FmtResult {
    match node.kind {
      DocNodeKind::ModuleDoc => self.format_module_doc(w, node, indent),
      DocNodeKind::Function => {
        self.format_function_signature(w, node, indent, has_overloads)
      }
      DocNodeKind::Variable => self.format_variable_signature(w, node, indent),
      DocNodeKind::Class => self.format_class_signature(w, node, indent),
      DocNodeKind::Enum => self.format_enum_signature(w, node, indent),
      DocNodeKind::Interface => {
        self.format_interface_signature(w, node, indent)
      }
      DocNodeKind::TypeAlias => {
        self.format_type_alias_signature(w, node, indent)
      }
      DocNodeKind::Namespace => {
        self.format_namespace_signature(w, node, indent)
      }
      DocNodeKind::Import => Ok(()),
    }
  }

  fn format_jsdoc(
    &self,
    w: &mut Formatter<'_>,
    js_doc: &JsDoc,
    indent: i64,
  ) -> FmtResult {
    if let Some(doc) = &js_doc.doc {
      for line in doc.lines() {
        writeln!(w, "{}{}", Indent(indent), colors::gray(line))?;
      }
    }
    if !js_doc.tags.is_empty() {
      writeln!(w)?;
    }
    for tag in &js_doc.tags {
      self.format_jsdoc_tag(w, tag, indent)?;
    }
    Ok(())
  }

  fn format_jsdoc_tag_maybe_doc(
    &self,
    w: &mut Formatter<'_>,
    maybe_doc: &Option<String>,
    indent: i64,
  ) -> FmtResult {
    if let Some(doc) = maybe_doc {
      for line in doc.lines() {
        writeln!(w, "{}{}", Indent(indent + 2), colors::gray(line))?;
      }
      writeln!(w)
    } else {
      Ok(())
    }
  }

  fn format_jsdoc_tag(
    &self,
    w: &mut Formatter<'_>,
    tag: &JsDocTag,
    indent: i64,
  ) -> FmtResult {
    match tag {
      JsDocTag::Callback { name, doc } => {
        writeln!(
          w,
          "{}@{} {}",
          Indent(indent),
          colors::magenta("callback"),
          colors::bold(name)
        )?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Category { doc } => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta("category"))?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Constructor => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta("constructor"))
      }
      JsDocTag::Default { value, doc } => {
        writeln!(
          w,
          "{}@{} {{{}}}",
          Indent(indent),
          colors::magenta("default"),
          colors::italic_cyan(value)
        )?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Deprecated { doc } => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta("deprecated"))?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Enum { type_ref, doc } => {
        writeln!(
          w,
          "{}@{} {{{}}}",
          Indent(indent),
          colors::magenta("enum"),
          colors::italic_cyan(type_ref)
        )?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Example { doc } => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta("example"))?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Extends { type_ref, doc } => {
        writeln!(
          w,
          "{}@{} {{{}}}",
          Indent(indent),
          colors::magenta("extends"),
          colors::italic_cyan(type_ref)
        )?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Ignore => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta("ignore"))
      }
      JsDocTag::Module => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta("module"))
      }
      JsDocTag::Param {
        name,
        type_ref,
        optional,
        default,
        doc,
      } => {
        write!(w, "{}@{}", Indent(indent), colors::magenta("param"))?;
        if let Some(type_ref) = type_ref {
          write!(w, " {{{}}}", colors::italic_cyan(type_ref))?;
        }
        if *optional {
          write!(w, " [?]")?;
        } else if let Some(default) = default {
          write!(w, " [{}]", colors::italic_cyan(default))?;
        }
        writeln!(w, " {}", colors::bold(name))?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Public => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta("public"))
      }
      JsDocTag::Private => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta("private"))
      }
      JsDocTag::Property {
        name,
        type_ref,
        doc,
      } => {
        writeln!(
          w,
          "{}@{} {{{}}} {}",
          Indent(indent),
          colors::magenta("property"),
          colors::italic_cyan(type_ref),
          colors::bold(name)
        )?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Protected => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta("protected"))
      }
      JsDocTag::ReadOnly => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta("readonly"))
      }
      JsDocTag::Return { type_ref, doc } => {
        write!(w, "{}@{}", Indent(indent), colors::magenta("return"))?;
        if let Some(type_ref) = type_ref {
          writeln!(w, " {{{}}}", colors::italic_cyan(type_ref))?;
        } else {
          writeln!(w)?;
        }
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Tags { tags } => {
        writeln!(
          w,
          "{}@{} {}",
          Indent(indent),
          colors::magenta("tags"),
          tags.join(", "),
        )
      }
      JsDocTag::Template { name, doc } => {
        writeln!(
          w,
          "{}@{} {}",
          Indent(indent),
          colors::magenta("template"),
          colors::bold(name)
        )?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::This { type_ref, doc } => {
        writeln!(
          w,
          "{}@{} {{{}}}",
          Indent(indent),
          colors::magenta("this"),
          colors::italic_cyan(type_ref)
        )?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::TypeDef {
        name,
        type_ref,
        doc,
      } => {
        writeln!(
          w,
          "{}@{} {{{}}} {}",
          Indent(indent),
          colors::magenta("typedef"),
          colors::italic_cyan(type_ref),
          colors::bold(name)
        )?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::TypeRef { type_ref, doc } => {
        writeln!(
          w,
          "{}@{} {{{}}}",
          Indent(indent),
          colors::magenta("typeref"),
          colors::italic_cyan(type_ref)
        )?;
        self.format_jsdoc_tag_maybe_doc(w, doc, indent)
      }
      JsDocTag::Unsupported { value } => {
        writeln!(w, "{}@{}", Indent(indent), colors::magenta(value))
      }
    }
  }

  fn format_class(&self, w: &mut Formatter<'_>, node: &DocNode) -> FmtResult {
    let class_def = node.class_def.as_ref().unwrap();
    let has_overloads = class_def.constructors.len() > 1;
    for node in &class_def.constructors {
      if !has_overloads || !node.has_body {
        writeln!(w, "{}{}", Indent(1), node,)?;
        self.format_jsdoc(w, &node.js_doc, 2)?;
      }
    }
    for node in class_def.properties.iter().filter(|node| {
      self.private
        || node
          .accessibility
          .unwrap_or(deno_ast::swc::ast::Accessibility::Public)
          != deno_ast::swc::ast::Accessibility::Private
    }) {
      for d in &node.decorators {
        writeln!(w, "{}{}", Indent(1), d)?;
      }
      writeln!(w, "{}{}", Indent(1), node,)?;
      self.format_jsdoc(w, &node.js_doc, 2)?;
    }
    for index_sign_def in &class_def.index_signatures {
      writeln!(w, "{}{}", Indent(1), index_sign_def)?;
    }
    for node in class_def.methods.iter().filter(|node| {
      self.private
        || node
          .accessibility
          .unwrap_or(deno_ast::swc::ast::Accessibility::Public)
          != deno_ast::swc::ast::Accessibility::Private
    }) {
      let has_overloads = class_def
        .methods
        .iter()
        .filter(|n| n.name == node.name)
        .count()
        > 1;
      if !has_overloads || !node.function_def.has_body {
        for d in &node.function_def.decorators {
          writeln!(w, "{}{}", Indent(1), d)?;
        }
        writeln!(w, "{}{}", Indent(1), node,)?;
        self.format_jsdoc(w, &node.js_doc, 2)?;
      }
    }
    writeln!(w)
  }

  fn format_enum(&self, w: &mut Formatter<'_>, node: &DocNode) -> FmtResult {
    let enum_def = node.enum_def.as_ref().unwrap();
    for member in &enum_def.members {
      writeln!(w, "{}{}", Indent(1), colors::bold(&member.name))?;
      self.format_jsdoc(w, &member.js_doc, 2)?;
    }
    writeln!(w)
  }

  fn format_interface(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
  ) -> FmtResult {
    let interface_def = node.interface_def.as_ref().unwrap();

    for property_def in &interface_def.properties {
      writeln!(w, "{}{}", Indent(1), property_def)?;
      self.format_jsdoc(w, &property_def.js_doc, 2)?;
    }
    for method_def in &interface_def.methods {
      writeln!(w, "{}{}", Indent(1), method_def)?;
      self.format_jsdoc(w, &method_def.js_doc, 2)?;
    }
    for index_sign_def in &interface_def.index_signatures {
      writeln!(w, "{}{}", Indent(1), index_sign_def)?;
    }
    writeln!(w)
  }

  fn format_namespace(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
  ) -> FmtResult {
    let elements = &node.namespace_def.as_ref().unwrap().elements;
    for node in elements {
      let has_overloads = if node.kind == DocNodeKind::Function {
        elements
          .iter()
          .filter(|n| n.kind == DocNodeKind::Function && n.name == node.name)
          .count()
          > 1
      } else {
        false
      };
      self.format_signature(w, node, 1, has_overloads)?;
      self.format_jsdoc(w, &node.js_doc, 2)?;
    }
    writeln!(w)
  }

  fn format_class_signature(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
    indent: i64,
  ) -> FmtResult {
    let class_def = node.class_def.as_ref().unwrap();
    for node in &class_def.decorators {
      writeln!(w, "{}{}", Indent(indent), node)?;
    }
    write!(
      w,
      "{}{}{} {}",
      Indent(indent),
      display_abstract(class_def.is_abstract),
      colors::magenta("class"),
      colors::bold(&node.name),
    )?;
    if !class_def.type_params.is_empty() {
      write!(
        w,
        "<{}>",
        SliceDisplayer::new(&class_def.type_params, ", ", false)
      )?;
    }

    if let Some(extends) = &class_def.extends {
      write!(w, " {} {}", colors::magenta("extends"), extends)?;
    }
    if !class_def.super_type_params.is_empty() {
      write!(
        w,
        "<{}>",
        SliceDisplayer::new(&class_def.super_type_params, ", ", false)
      )?;
    }

    if !class_def.implements.is_empty() {
      write!(
        w,
        " {} {}",
        colors::magenta("implements"),
        SliceDisplayer::new(&class_def.implements, ", ", false)
      )?;
    }

    writeln!(w)
  }

  fn format_enum_signature(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
    indent: i64,
  ) -> FmtResult {
    writeln!(
      w,
      "{}{} {}",
      Indent(indent),
      colors::magenta("enum"),
      colors::bold(&node.name)
    )
  }

  fn format_function_signature(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
    indent: i64,
    has_overloads: bool,
  ) -> FmtResult {
    let function_def = node.function_def.as_ref().unwrap();
    if !has_overloads || !function_def.has_body {
      write!(
        w,
        "{}{}{}{} {}",
        Indent(indent),
        display_async(function_def.is_async),
        colors::magenta("function"),
        display_generator(function_def.is_generator),
        colors::bold(&node.name)
      )?;
      if !function_def.type_params.is_empty() {
        write!(
          w,
          "<{}>",
          SliceDisplayer::new(&function_def.type_params, ", ", false)
        )?;
      }
      write!(
        w,
        "({})",
        SliceDisplayer::new(&function_def.params, ", ", false)
      )?;
      if let Some(return_type) = &function_def.return_type {
        write!(w, ": {}", return_type)?;
      }
      writeln!(w)?;
    }
    Ok(())
  }

  fn format_interface_signature(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
    indent: i64,
  ) -> FmtResult {
    let interface_def = node.interface_def.as_ref().unwrap();
    write!(
      w,
      "{}{} {}",
      Indent(indent),
      colors::magenta("interface"),
      colors::bold(&node.name)
    )?;

    if !interface_def.type_params.is_empty() {
      write!(
        w,
        "<{}>",
        SliceDisplayer::new(&interface_def.type_params, ", ", false)
      )?;
    }

    if !interface_def.extends.is_empty() {
      write!(
        w,
        " {} {}",
        colors::magenta("extends"),
        SliceDisplayer::new(&interface_def.extends, ", ", false)
      )?;
    }

    writeln!(w)
  }

  fn format_module_doc(
    &self,
    _w: &mut Formatter<'_>,
    _node: &DocNode,
    _indent: i64,
  ) -> FmtResult {
    // currently we do not print out JSDoc in the printer, so there is nothing
    // to print.
    Ok(())
  }

  fn format_type_alias_signature(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
    indent: i64,
  ) -> FmtResult {
    let type_alias_def = node.type_alias_def.as_ref().unwrap();
    write!(
      w,
      "{}{} {}",
      Indent(indent),
      colors::magenta("type"),
      colors::bold(&node.name),
    )?;

    if !type_alias_def.type_params.is_empty() {
      write!(
        w,
        "<{}>",
        SliceDisplayer::new(&type_alias_def.type_params, ", ", false)
      )?;
    }

    writeln!(w, " = {}", type_alias_def.ts_type)
  }

  fn format_namespace_signature(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
    indent: i64,
  ) -> FmtResult {
    writeln!(
      w,
      "{}{} {}",
      Indent(indent),
      colors::magenta("namespace"),
      colors::bold(&node.name)
    )
  }

  fn format_variable_signature(
    &self,
    w: &mut Formatter<'_>,
    node: &DocNode,
    indent: i64,
  ) -> FmtResult {
    let variable_def = node.variable_def.as_ref().unwrap();
    write!(
      w,
      "{}{} {}",
      Indent(indent),
      colors::magenta(match variable_def.kind {
        deno_ast::swc::ast::VarDeclKind::Const => "const",
        deno_ast::swc::ast::VarDeclKind::Let => "let",
        deno_ast::swc::ast::VarDeclKind::Var => "var",
      }),
      colors::bold(&node.name),
    )?;
    if let Some(ts_type) = &variable_def.ts_type {
      write!(w, ": {}", ts_type)?;
    }
    writeln!(w)
  }
}
