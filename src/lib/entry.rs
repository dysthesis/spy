use std::{collections::HashSet, fmt::Display};

use serde_json::Value;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;
use uuid::Uuid;

use crate::AGENT;

#[derive(Debug, Serialize, Deserialize, Clone)]
/// A single bookmark entry.
pub struct Entry {
    id: Uuid,
    url: Url,
    page_title: String,
    site_title: String,
    authors: HashSet<String>,
    full_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumbnail: Option<Url>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to fetch URL {url}")]
    FetchError { error: ureq::Error, url: Url },
    #[error("Failed to read to string: {url}")]
    ReadToStringError { error: ureq::Error, url: Url },
}

impl Entry {
    /// Construct a new Entry from a Url, and optionally, a user-defined title.
    pub fn new(url: &Url, page_title: Option<String>) -> Result<Self, Box<Error>> {
        let body = AGENT
            .get(url.as_str())
            .call()
            .map_err(|e| Error::FetchError {
                error: e,
                url: url.clone(),
            })?
            .body_mut()
            .read_to_string()
            .map_err(|e| Error::ReadToStringError {
                error: e,
                url: url.clone(),
            })?;
        let doc = Html::parse_document(&body);
        let mut bytes = body.as_bytes();
        let full_text = readability::extractor::extract(&mut bytes, url)
            .map(|p| p.content)
            .unwrap_or_default();
        let page_title = page_title
            .or_else(|| first_text(&doc, "head > title"))
            .or_else(|| first_attr(&doc, r#"head meta[property="og:title"]"#, "content"))
            .or_else(|| first_attr(&doc, r#"head meta[name="twitter:title"]"#, "content"))
            .or_else(|| json_ld_title(&doc))
            .or_else(|| {
                first_attr(&doc, r#"[itemprop="headline"]"#, "content")
                    .or_else(|| first_text(&doc, r#"[itemprop="headline"]"#))
                    .or_else(|| first_attr(&doc, r#"[itemprop="name"]"#, "content"))
                    .or_else(|| first_text(&doc, r#"[itemprop="name"]"#))
            })
            .or_else(|| {
                first_text(&doc, ".h-entry .p-name")
                    .or_else(|| first_text(&doc, ".p-name"))
                    .or_else(|| first_text(&doc, ".h-entry .entry-title"))
            })
            .or_else(|| {
                first_attr(&doc, r#"[property="schema:headline"]"#, "content")
                    .or_else(|| first_text(&doc, r#"[property="schema:headline"]"#))
                    .or_else(|| first_attr(&doc, r#"[property="schema:name"]"#, "content"))
                    .or_else(|| first_text(&doc, r#"[property="schema:name"]"#))
                    .or_else(|| first_attr(&doc, r#"[property="dcterms:title"]"#, "content"))
                    .or_else(|| first_text(&doc, r#"[property="dcterms:title"]"#))
            })
            .or_else(|| dublin_core_meta(&doc))
            .unwrap_or_default();

        let site_title = og_site_name(&doc)
            .or_else(|| manifest_site_name(url, &doc))
            .or_else(|| schema_site_name(&doc))
            .or_else(|| microformats_site_name(&doc))
            .or_else(|| meta_application_name(&doc))
            .or_else(|| url.host_str().map(str::to_string))
            .unwrap_or_default();

        let authors = meta_author(&doc)
            .or_else(|| link_rel_author(&doc))
            .or_else(|| json_ld_authors(&doc))
            .or_else(|| microdata_authors(&doc))
            .or_else(|| rdfa_authors(&doc))
            .or_else(|| microformats_authors(&doc))
            .or_else(|| og_article_authors(&doc))
            .or_else(|| twitter_creator(&doc))
            .or_else(|| dublin_core_creators(&doc))
            .or_else(|| address_authors(&doc))
            .unwrap_or_default();
        let description = meta_description(&doc)
            .or_else(|| og_description(&doc))
            .or_else(|| twitter_description(&doc))
            .or_else(|| schema_description_jsonld(&doc))
            .or_else(|| schema_description_microdata_rdfa(&doc))
            .or_else(|| microformats_summary(&doc))
            .or_else(|| dublin_core_description(&doc))
            .or_else(|| manifest_description(url, &doc));
        let thumbnail = og_image(url, &doc)
            .or_else(|| twitter_image(url, &doc))
            .or_else(|| schema_primary_image_jsonld(url, &doc))
            .or_else(|| schema_primary_image_microdata_rdfa(url, &doc))
            .or_else(|| schema_image_jsonld(url, &doc))
            .or_else(|| schema_image_microdata_rdfa(url, &doc))
            .or_else(|| microformats_image(url, &doc))
            .or_else(|| oembed_thumbnail(url, &doc))
            .or_else(|| amp_story_poster(url, &doc))
            .or_else(|| rel_image_src(url, &doc))
            .and_then(|s| Url::parse(&s).ok());

        let id = Uuid::new_v4();
        Ok(Entry {
            id,
            url: url.clone(),
            page_title,
            site_title,
            authors,
            description,
            full_text,
            thumbnail,
        })
    }
}

fn first_text(doc: &Html, css: &str) -> Option<String> {
    let sel = Selector::parse(css).ok()?;
    doc.select(&sel)
        .next()
        .map(|e| e.text().collect::<String>())
        .map(|s| collapse_ws(&s))
        .filter(|s| !s.is_empty())
}

fn first_attr(doc: &Html, css: &str, attr: &str) -> Option<String> {
    let sel = Selector::parse(css).ok()?;
    doc.select(&sel)
        .filter_map(|e| e.value().attr(attr))
        .map(collapse_ws)
        .find(|s| !s.is_empty())
}

fn json_ld_title(doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"script[type="application/ld+json"]"#).ok()?;
    let mut cands = Vec::<String>::new();
    for node in doc.select(&sel) {
        let raw = node.text().collect::<String>();
        if let Ok(val) = serde_json::from_str::<Value>(&raw) {
            collect_schema_titles(&val, &mut cands);
        }
    }
    cands
        .into_iter()
        .map(|s| collapse_ws(&s))
        .find(|s| !s.is_empty())
}

fn collect_schema_titles(v: &serde_json::Value, out: &mut Vec<String>) {
    use serde_json::Value::*;
    match v {
        Object(m) => {
            for key in ["headline", "name", "alternativeHeadline"] {
                if let Some(String(s)) = m.get(key) {
                    let s = s.trim();
                    if !s.is_empty() {
                        out.push(s.to_owned());
                    }
                }
            }
            if let Some(g) = m.get("@graph") {
                collect_schema_titles(g, out);
            }
            for (_k, vv) in m {
                collect_schema_titles(vv, out);
            }
        }
        Array(a) => {
            for x in a {
                collect_schema_titles(x, out);
            }
        }
        _ => {}
    }
}

fn dublin_core_meta(doc: &Html) -> Option<String> {
    let sel = Selector::parse("head meta").ok()?;
    for m in doc.select(&sel) {
        let name = m.value().attr("name").unwrap_or_default();
        let lname = name.to_ascii_lowercase();
        if lname == "dc.title" || lname == "dcterms.title" {
            if let Some(val) = m.value().attr("content") {
                let s = collapse_ws(val);
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut was_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !was_space {
                out.push(' ');
            }
            was_space = true;
        } else {
            out.push(ch);
            was_space = false;
        }
    }
    out.trim().to_owned()
}

fn og_site_name(doc: &Html) -> Option<String> {
    first_attr(doc, r#"head meta[property="og:site_name"]"#, "content")
}

fn manifest_site_name(base: &Url, doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"link[rel~="manifest"]"#).ok()?;
    let href = doc
        .select(&sel)
        .filter_map(|l| l.value().attr("href"))
        .next()?;
    let manifest_url = base.join(href).ok()?;
    let resp = AGENT.get(manifest_url.as_str()).call().ok()?;
    let text = resp.into_body().read_to_string().ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("name")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .or_else(|| {
            v.get("short_name")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        })
}

fn schema_site_name(doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"script[type="application/ld+json"]"#).ok()?;
    let mut cands = Vec::<String>::new();
    for node in doc.select(&sel) {
        let raw = node.text().collect::<String>();
        if let Ok(val) = serde_json::from_str::<Value>(&raw) {
            collect_schema_site_names(&val, &mut cands);
        }
    }
    if let Some(s) = cands
        .into_iter()
        .map(|s| collapse_ws(&s))
        .find(|s| !s.is_empty())
    {
        return Some(s);
    }
    first_attr(
        doc,
        r#"[itemscope][itemtype*="schema.org/WebSite"] [itemprop="name"]"#,
        "content",
    )
    .or_else(|| {
        first_text(
            doc,
            r#"[itemscope][itemtype*="schema.org/WebSite"] [itemprop="name"]"#,
        )
    })
    .or_else(|| {
        first_attr(
            doc,
            r#"[itemscope][itemtype*="schema.org/Organization"] [itemprop="name"]"#,
            "content",
        )
    })
    .or_else(|| {
        first_text(
            doc,
            r#"[itemscope][itemtype*="schema.org/Organization"] [itemprop="name"]"#,
        )
    })
    .or_else(|| first_attr(doc, r#"[property="schema:name"]"#, "content"))
    .or_else(|| first_text(doc, r#"[property="schema:name"]"#))
}

fn collect_schema_site_names(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            if let Some(Value::String(t)) = m.get("@type") {
                if t.contains("WebSite") {
                    if let Some(Value::String(n)) = m.get("name") {
                        if !n.trim().is_empty() {
                            out.push(n.trim().to_owned());
                        }
                    }
                }
            }
            if let Some(pub_obj) = m.get("publisher") {
                if let Some(name) = pub_obj.get("name").and_then(|x| x.as_str()) {
                    if !name.trim().is_empty() {
                        out.push(name.trim().to_owned());
                    }
                }
                collect_schema_site_names(pub_obj, out);
            }
            if let Some(g) = m.get("@graph") {
                collect_schema_site_names(g, out);
            }
            for (_k, vv) in m {
                collect_schema_site_names(vv, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                collect_schema_site_names(x, out);
            }
        }
        _ => {}
    }
}

fn microformats_site_name(doc: &Html) -> Option<String> {
    first_text(doc, ".h-card .p-name").or_else(|| first_text(doc, ".p-name"))
}

fn meta_application_name(doc: &Html) -> Option<String> {
    first_attr(doc, r#"head meta[name="application-name"]"#, "content")
}

fn meta_author(doc: &Html) -> Option<HashSet<String>> {
    first_attr_all(doc, r#"head meta[name="author"]"#, "content")
}

fn link_rel_author(doc: &Html) -> Option<HashSet<String>> {
    let mut out = HashSet::new();
    for css in [r#"a[rel~="author"]"#, r#"link[rel~="author"]"#] {
        let sel = Selector::parse(css).ok()?;
        for el in doc.select(&sel) {
            let text = collapse_ws(&el.text().collect::<String>());
            if !text.is_empty() {
                out.insert(text);
            } else if let Some(title) = el.value().attr("title") {
                let s = collapse_ws(title);
                if !s.is_empty() {
                    out.insert(s);
                }
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn json_ld_authors(doc: &Html) -> Option<HashSet<String>> {
    use serde_json::Value;
    let sel = Selector::parse(r#"script[type="application/ld+json"]"#).ok()?;
    let mut out = HashSet::new();
    for node in doc.select(&sel) {
        let raw = node.text().collect::<String>();
        if let Ok(val) = serde_json::from_str::<Value>(&raw) {
            collect_schema_authors(&val, &mut out);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn microdata_authors(doc: &Html) -> Option<HashSet<String>> {
    let mut out = HashSet::new();

    if let Some(vals) = first_attr_all(doc, r#"[itemprop="author"]"#, "content") {
        out.extend(vals);
    }

    let sel = Selector::parse(r#"[itemprop="author"]"#).ok()?;
    for el in doc.select(&sel) {
        let text = collapse_ws(&el.text().collect::<String>());
        if !text.is_empty() {
            out.insert(text);
        }
        if let Some(names) = first_attr_all_in(&el, r#"[itemprop="name"]"#, "content") {
            out.extend(names);
        }
        let sel_name = Selector::parse(r#"[itemprop="name"]"#).unwrap();
        for child in el.select(&sel_name) {
            let t = collapse_ws(&child.text().collect::<String>());
            if !t.is_empty() {
                out.insert(t);
            }
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn rdfa_authors(doc: &Html) -> Option<HashSet<String>> {
    let mut out = HashSet::new();

    if let Some(vals) = first_attr_all(doc, r#"[property="schema:author"]"#, "content") {
        out.extend(vals);
    }
    for css in [
        r#"[property="schema:author"]"#,
        r#"[property="schema:name"]"#,
    ] {
        let sel = Selector::parse(css).ok()?;
        for el in doc.select(&sel) {
            let t = collapse_ws(&el.text().collect::<String>());
            if !t.is_empty() {
                out.insert(t);
            }
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn microformats_authors(doc: &Html) -> Option<HashSet<String>> {
    let mut out = HashSet::new();
    for css in [
        ".h-entry .p-author",
        ".p-author",
        ".h-entry .author",
        ".author.vcard",
    ] {
        let sel = Selector::parse(css).ok()?;
        for el in doc.select(&sel) {
            if let Some(n) = first_text_in(&el, ".p-name") {
                out.insert(n);
            }
            let t = collapse_ws(&el.text().collect::<String>());
            if !t.is_empty() {
                out.insert(t);
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn og_article_authors(doc: &Html) -> Option<HashSet<String>> {
    first_attr_all(doc, r#"head meta[property="article:author"]"#, "content")
}

fn twitter_creator(doc: &Html) -> Option<HashSet<String>> {
    first_attr_all(doc, r#"head meta[name="twitter:creator"]"#, "content")
        .map(|set| set.into_iter().map(|s| trim_at(&s)).collect())
}

fn dublin_core_creators(doc: &Html) -> Option<HashSet<String>> {
    let mut out = HashSet::new();
    let sel = Selector::parse("head meta").ok()?;
    for m in doc.select(&sel) {
        if let Some(name) = m.value().attr("name") {
            let lname = name.to_ascii_lowercase();
            if lname == "dc.creator" || lname == "dcterms.creator" {
                if let Some(val) = m.value().attr("content") {
                    let s = collapse_ws(val);
                    if !s.is_empty() {
                        out.insert(s);
                    }
                }
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn address_authors(doc: &Html) -> Option<HashSet<String>> {
    let mut out = HashSet::new();

    for css in ["article address", "footer address", "address"] {
        let sel = Selector::parse(css).ok()?;
        for el in doc.select(&sel) {
            let t = collapse_ws(&el.text().collect::<String>());
            if !t.is_empty() {
                out.insert(t);
            }
        }
        if !out.is_empty() {
            break;
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn first_attr_all(doc: &Html, css: &str, attr: &str) -> Option<HashSet<String>> {
    let sel = Selector::parse(css).ok()?;
    let mut out = HashSet::new();
    for e in doc.select(&sel) {
        if let Some(v) = e.value().attr(attr) {
            let s = collapse_ws(v);
            if !s.is_empty() {
                out.insert(s);
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn first_attr_all_in(
    el: &scraper::element_ref::ElementRef,
    css: &str,
    attr: &str,
) -> Option<HashSet<String>> {
    let sel = Selector::parse(css).ok()?;
    let mut out = HashSet::new();
    for c in el.select(&sel) {
        if let Some(v) = c.value().attr(attr) {
            let s = collapse_ws(v);
            if !s.is_empty() {
                out.insert(s);
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn first_text_in(el: &scraper::element_ref::ElementRef, css: &str) -> Option<String> {
    let sel = Selector::parse(css).ok()?;
    el.select(&sel)
        .next()
        .map(|e| collapse_ws(&e.text().collect::<String>()))
        .filter(|s| !s.is_empty())
}

fn trim_at(s: &str) -> String {
    s.trim().trim_start_matches('@').to_string()
}

pub fn collect_schema_authors(v: &Value, out: &mut HashSet<String>) {
    match v {
        Value::Object(m) => {
            if let Some(a) = m.get("author") {
                extract_author_node(a, out);
            }
            if let Some(a) = m.get("creator") {
                extract_author_node(a, out);
            }

            if let Some(g) = m.get("@graph") {
                collect_schema_authors(g, out);
            }

            for (_k, vv) in m {
                collect_schema_authors(vv, out);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_schema_authors(item, out);
            }
        }
        _ => {}
    }
}

fn extract_author_node(node: &Value, out: &mut HashSet<String>) {
    match node {
        Value::String(s) => {
            let s = s.trim();
            if !s.is_empty() && !looks_like_url(s) {
                out.insert(s.to_owned());
            }
        }
        Value::Object(m) => {
            if let Some(Value::String(n)) = m.get("name") {
                let n = n.trim();
                if !n.is_empty() {
                    out.insert(n.to_owned());
                    return;
                }
            }
            let gn = m
                .get("givenName")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            let fn_ = m
                .get("familyName")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if !gn.is_empty() || !fn_.is_empty() {
                let mut full = String::new();
                if !gn.is_empty() {
                    full.push_str(gn);
                }
                if !fn_.is_empty() {
                    if !full.is_empty() {
                        full.push(' ');
                    }
                    full.push_str(fn_);
                }
                if !full.is_empty() {
                    out.insert(full);
                }
            }
        }
        Value::Array(arr) => {
            for a in arr {
                extract_author_node(a, out);
            }
        }
        _ => {}
    }
}

fn looks_like_url(s: &str) -> bool {
    let ls = s.to_ascii_lowercase();
    ls.starts_with("http://") || ls.starts_with("https://")
}

fn meta_description(doc: &Html) -> Option<String> {
    let sel = Selector::parse("head meta").ok()?;
    for m in doc.select(&sel) {
        if let Some(name) = m.value().attr("name") {
            if name.eq_ignore_ascii_case("description") {
                if let Some(val) = m.value().attr("content") {
                    let s = collapse_ws(val);
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
    }
    None
}

fn og_description(doc: &Html) -> Option<String> {
    first_attr(doc, r#"head meta[property="og:description"]"#, "content")
}

fn twitter_description(doc: &Html) -> Option<String> {
    first_attr(doc, r#"head meta[name="twitter:description"]"#, "content")
}

fn schema_description_jsonld(doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"script[type="application/ld+json"]"#).ok()?;
    let mut cands = Vec::<String>::new();
    for node in doc.select(&sel) {
        let raw = node.text().collect::<String>();
        if let Ok(val) = serde_json::from_str::<Value>(&raw) {
            collect_schema_descriptions(&val, &mut cands);
        }
    }
    cands
        .into_iter()
        .map(|s| collapse_ws(&s))
        .find(|s| !s.is_empty())
}

fn schema_description_microdata_rdfa(doc: &Html) -> Option<String> {
    first_attr(doc, r#"[itemprop="description"]"#, "content")
        .or_else(|| first_text(doc, r#"[itemprop="description"]"#))
        .or_else(|| first_attr(doc, r#"[itemprop="abstract"]"#, "content"))
        .or_else(|| first_text(doc, r#"[itemprop="abstract"]"#))
        .or_else(|| first_attr(doc, r#"[property="schema:description"]"#, "content"))
        .or_else(|| first_text(doc, r#"[property="schema:description"]"#))
}

fn microformats_summary(doc: &Html) -> Option<String> {
    first_text(doc, ".h-entry .p-summary").or_else(|| first_text(doc, ".p-summary"))
}

fn dublin_core_description(doc: &Html) -> Option<String> {
    let sel = Selector::parse("head meta").ok()?;
    for m in doc.select(&sel) {
        if let Some(name) = m.value().attr("name") {
            let lname = name.to_ascii_lowercase();
            if lname == "dc.description"
                || lname == "dcterms.description"
                || lname == "dcterms.abstract"
            {
                if let Some(val) = m.value().attr("content") {
                    let s = collapse_ws(val);
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
    }
    None
}

fn manifest_description(base: &Url, doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"link[rel~="manifest"]"#).ok()?;
    let href = doc
        .select(&sel)
        .filter_map(|l| l.value().attr("href"))
        .next()?;
    let manifest_url = base.join(href).ok()?;

    let text = crate::AGENT
        .get(manifest_url.as_str())
        .call()
        .ok()?
        .into_body()
        .read_to_string()
        .ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("description")
        .and_then(Value::as_str)
        .map(collapse_ws)
        .filter(|s| !s.is_empty())
}

fn collect_schema_descriptions(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            for key in ["description", "abstract"] {
                if let Some(Value::String(s)) = m.get(key) {
                    let t = s.trim();
                    if !t.is_empty() {
                        out.push(t.to_owned());
                    }
                }
            }
            if let Some(g) = m.get("@graph") {
                collect_schema_descriptions(g, out);
            }
            for (_k, vv) in m {
                collect_schema_descriptions(vv, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                collect_schema_descriptions(x, out);
            }
        }
        _ => {}
    }
}

fn og_image(base: &Url, doc: &Html) -> Option<String> {
    for css in [
        r#"head meta[property="og:image:secure_url"]"#, // prefer https when given
        r#"head meta[property="og:image:url"]"#,
        r#"head meta[property="og:image"]"#,
    ] {
        if let Some(u) = first_attr(doc, css, "content") {
            if let Some(abs) = absolutise(base, &u) {
                return Some(abs);
            }
        }
    }
    None
}

fn twitter_image(base: &Url, doc: &Html) -> Option<String> {
    for css in [
        r#"head meta[name="twitter:image"]"#,
        r#"head meta[name="twitter:image:src"]"#, // seen in the wild
    ] {
        if let Some(u) = first_attr(doc, css, "content") {
            if let Some(abs) = absolutise(base, &u) {
                return Some(abs);
            }
        }
    }
    None
}

fn schema_primary_image_jsonld(base: &Url, doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"script[type="application/ld+json"]"#).ok()?;
    let mut cands = Vec::<String>::new();
    for node in doc.select(&sel) {
        let raw = node.text().collect::<String>();
        if let Ok(val) = serde_json::from_str::<Value>(&raw) {
            collect_primary_image(&val, &mut cands); // WebPage.primaryImageOfPage first
        }
    }
    cands
        .into_iter()
        .filter_map(|u| absolutise(base, &u))
        .next()
}

fn schema_image_jsonld(base: &Url, doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"script[type="application/ld+json"]"#).ok()?;
    let mut cands = Vec::<String>::new();
    for node in doc.select(&sel) {
        let raw = node.text().collect::<String>();
        if let Ok(val) = serde_json::from_str::<Value>(&raw) {
            collect_generic_image(&val, &mut cands); // CreativeWork.image as fallback
        }
    }
    cands
        .into_iter()
        .filter_map(|u| absolutise(base, &u))
        .next()
}

fn schema_primary_image_microdata_rdfa(base: &Url, doc: &Html) -> Option<String> {
    for css in [
        r#"[itemprop="primaryImageOfPage"]"#,
        r#"[property="schema:primaryImageOfPage"]"#,
    ] {
        if let Some(u) = url_from_any_attr(doc, css) {
            if let Some(abs) = absolutise(base, &u) {
                return Some(abs);
            }
        }
    }
    None
}

fn schema_image_microdata_rdfa(base: &Url, doc: &Html) -> Option<String> {
    for css in [r#"[itemprop="image"]"#, r#"[property="schema:image"]"#] {
        if let Some(u) = url_from_any_attr(doc, css) {
            if let Some(abs) = absolutise(base, &u) {
                return Some(abs);
            }
        }
    }
    None
}

fn microformats_image(base: &Url, doc: &Html) -> Option<String> {
    for css in [
        ".h-entry .u-featured",
        ".u-featured",
        ".h-entry .u-photo",
        ".u-photo",
    ] {
        if let Some(u) = url_from_any_attr(doc, css) {
            if let Some(abs) = absolutise(base, &u) {
                return Some(abs);
            }
        }
    }
    None
}

fn oembed_thumbnail(base: &Url, doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"link[rel~="alternate"]"#).ok()?;
    // Find an oEmbed endpoint advertised in <head>.
    let href = doc.select(&sel).find_map(|l| {
        let t = l.value().attr("type")?.to_ascii_lowercase();
        if t == "application/json+oembed" || t == "text/xml+oembed" {
            l.value().attr("href").map(|h| h.to_string())
        } else {
            None
        }
    })?;
    // Fetch JSON oEmbed only (keep simple). If XML, you could parse with quick-xml.
    let oembed_url = base.join(&href).ok()?;
    let body = crate::AGENT
        .get(oembed_url.as_str())
        .call()
        .ok()?
        .into_body()
        .read_to_string()
        .ok()?;
    if href.contains("json+oembed") {
        if let Ok(v) = serde_json::from_str::<Value>(&body) {
            if let Some(u) = v
                .get("thumbnail_url")
                .and_then(Value::as_str)
                .or_else(|| v.get("url").and_then(Value::as_str))
            // photo type uses "url"
            {
                return absolutise(base, u);
            }
        }
    }
    None
}

fn amp_story_poster(base: &Url, doc: &Html) -> Option<String> {
    let sel = Selector::parse("amp-story").ok()?;
    let el = doc.select(&sel).next()?;
    for attr in [
        "poster-portrait-src",
        "poster-landscape-src",
        "poster-square-src",
    ] {
        if let Some(u) = el.value().attr(attr) {
            if let Some(abs) = absolutise(base, u) {
                return Some(abs);
            }
        }
    }
    None // poster-* are required for valid stories; return None if not present.
}

fn rel_image_src(base: &Url, doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"link[rel="image_src"]"#).ok()?;
    doc.select(&sel)
        .filter_map(|l| l.value().attr("href"))
        .filter_map(|u| absolutise(base, u))
        .next() // last resort only; not a standard.
}

fn collect_primary_image(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            // Only WebPage.primaryImageOfPage.
            if is_type(m, "WebPage") {
                if let Some(x) = m.get("primaryImageOfPage") {
                    push_image_value(x, out);
                }
            }
            if let Some(g) = m.get("@graph") {
                collect_primary_image(g, out);
            }
            for (_k, vv) in m {
                collect_primary_image(vv, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                collect_primary_image(x, out);
            }
        }
        _ => {}
    }
}

fn collect_generic_image(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            if let Some(x) = m.get("image") {
                // Thing/CreativeWork.image.
                // If the ImageObject says representativeOfPage=true, prefer it by inserting first.
                if let Some(img) = x.as_object() {
                    if img.get("representativeOfPage").and_then(Value::as_bool) == Some(true) {
                        let mut tmp = Vec::new();
                        push_image_value(x, &mut tmp);
                        out.splice(0..0, tmp); // stable prefer
                    } else {
                        push_image_value(x, out);
                    }
                } else {
                    push_image_value(x, out);
                }
            }
            if let Some(g) = m.get("@graph") {
                collect_generic_image(g, out);
            }
            for (_k, vv) in m {
                collect_generic_image(vv, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                collect_generic_image(x, out);
            }
        }
        _ => {}
    }
}

fn push_image_value(x: &Value, out: &mut Vec<String>) {
    match x {
        Value::String(s) => push_clean(s, out),
        Value::Object(m) => {
            if let Some(Value::String(u)) = m.get("contentUrl").or_else(|| m.get("url")) {
                push_clean(u, out);
            }
        }
        Value::Array(a) => {
            for v in a {
                push_image_value(v, out);
            }
        }
        _ => {}
    }
}

fn is_type(m: &serde_json::Map<String, Value>, t: &str) -> bool {
    match m.get("@type") {
        Some(Value::String(s)) => s == t,
        Some(Value::Array(arr)) => arr.iter().any(|v| v.as_str() == Some(t)),
        _ => false,
    }
}

fn push_clean(s: &str, out: &mut Vec<String>) {
    let u = s.trim();
    if !u.is_empty() {
        out.push(u.to_owned());
    }
}

fn url_from_any_attr(doc: &Html, css: &str) -> Option<String> {
    let sel = Selector::parse(css).ok()?;
    for e in doc.select(&sel) {
        for a in ["content", "src", "href", "data-src"] {
            if let Some(v) = e.value().attr(a) {
                let t = v.trim();
                if !t.is_empty() {
                    return Some(t.to_owned());
                }
            }
        }
    }
    None
}

fn absolutise(base: &Url, candidate: &str) -> Option<String> {
    let c = candidate.trim();
    // Avoid data URIs and fragments.
    if c.starts_with("data:") || c.starts_with('#') {
        return None;
    }
    // Already absolute http(s)?
    if let Ok(u) = Url::parse(c) {
        if u.scheme() == "http" || u.scheme() == "https" {
            return Some(u.into());
        }
        return None;
    }
    base.join(c).ok().map(|u| u.into())
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let view = EntryView::from(self);
        let json = serde_json::to_string(&view).map_err(|_| std::fmt::Error)?;
        f.write_str(&json)
    }
}
#[derive(Serialize)]
pub(crate) struct EntryView<'a> {
    #[serde(rename = "title")]
    title: &'a str,
    #[serde(rename = "site")]
    site: &'a str,

    // Primary author if any (deterministic: lexicographically smallest).
    #[serde(rename = "author", skip_serializing_if = "Option::is_none")]
    author: Option<String>,

    // Convenience: a comma-joined author list.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    authors: Vec<String>,

    url: &'a str,
    id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    thumbnail: Option<&'a str>,

    full_text: &'a str,
}

impl<'a> From<&'a Entry> for EntryView<'a> {
    fn from(e: &'a Entry) -> Self {
        // Authors: sort for determinism, then pick primary and build list.
        let mut authors: Vec<&str> = e.authors.iter().map(String::as_str).collect();
        authors.sort_unstable();
        let author = authors.first().map(|s| (*s).to_string());
        let authors_list = authors.iter().map(|s| (*s).to_string()).collect();

        EntryView {
            title: &e.page_title,
            site: &e.site_title,
            author,
            authors: authors_list,
            url: e.url.as_str(),
            id: e.id.to_string(),
            description: e.description.as_deref(),
            thumbnail: e.thumbnail.as_ref().map(|u| u.as_str()),
            full_text: &e.full_text,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct EntryTemplateContext<'a> {
    pub(crate) entry: &'a Entry,
    #[serde(flatten)]
    view: EntryView<'a>,
}

impl<'a> EntryTemplateContext<'a> {
    pub(crate) fn new(entry: &'a Entry) -> Self {
        Self {
            entry,
            view: EntryView::from(entry),
        }
    }
}
