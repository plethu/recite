//! Linkable workspace locations; no edits or manuscript content in URLs.
use freya::{prelude::*, router::*};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Screen {
    #[default]
    Write,
    Preview,
    Rules,
    Declarations,
    Disk,
    Rename,
    Build,
    Localise,
    Translations,
    Updates,
    Compare,
    Entry,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Location {
    pub screen: Screen,
    pub document: String,
    pub project: Option<String>,
    pub beat: Option<String>,
    pub passage: Option<String>,
    pub source: bool,
    pub view: Option<recite_config::WriterView>,
    pub catalogue: Option<String>,
    pub query: String,
    pub entry: Option<String>,
    pub attention: bool,
    pub page: usize,
}
impl Location {
    pub fn same_place(&self, other: &Self) -> bool {
        (self.screen != Screen::Rules || self.passage == other.passage)
            && self.project == other.project
            && self.screen == other.screen
            && self.document == other.document
            && self.beat == other.beat
            && self.source == other.source
            && self.view == other.view
            && self.catalogue == other.catalogue
            && self.entry == other.entry
    }
}
impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = match self.screen {
            Screen::Write => "/write",
            Screen::Preview => "/preview",
            Screen::Build => "/build",
            Screen::Rename => "/rename",
            Screen::Disk => "/disk-changes",
            Screen::Declarations => "/declarations",
            Screen::Rules => "/reply-rules",
            Screen::Localise => "/localise",
            Screen::Translations => "/translations",
            Screen::Updates => "/source-updates",
            Screen::Compare => "/external-changes",
            Screen::Entry => "/translation",
        };
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        if let Some(project) = &self.project {
            query.append_pair("project", project);
        }
        if !self.document.is_empty() {
            query.append_pair("scene", &self.document);
        }
        if let Some(entry) = &self.entry {
            query.append_pair("entry", entry);
        }
        if let Some(beat) = &self.beat {
            query.append_pair("beat", beat);
        }
        if let Some(passage) = &self.passage {
            query.append_pair("passage", passage);
        }
        if self.source {
            query.append_pair("view", "source");
        } else if let Some(view) = self.view {
            query.append_pair(
                "view",
                match view {
                    recite_config::WriterView::Script => "script",
                    recite_config::WriterView::Map => "map",
                    recite_config::WriterView::Source => "source",
                },
            );
        }
        if let Some(catalogue) = &self.catalogue {
            query.append_pair("catalogue", catalogue);
        }
        if !self.query.is_empty() {
            query.append_pair("q", &self.query);
        }
        if self.attention {
            query.append_pair("filter", "attention");
        }
        if self.page != 0 {
            query.append_pair("page", &self.page.to_string());
        }
        let query = query.finish();
        write!(
            f,
            "{path}{}{}",
            if query.is_empty() { "" } else { "?" },
            query
        )
    }
}
impl FromStr for Location {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let url = if value.starts_with('/') && !value.starts_with("//") {
            url::Url::parse(&format!("recite://writer{value}"))
        } else {
            url::Url::parse(value)
        }
        .map_err(|_| "Invalid writer link")?;
        if url.scheme() != "recite"
            || url.host_str() != Some("writer")
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
        {
            return Err("Invalid writer link");
        }
        let screen = match url.path() {
            "/" | "/write" => Screen::Write,
            "/preview" => Screen::Preview,
            "/build" => Screen::Build,
            "/rename" => Screen::Rename,
            "/disk-changes" => Screen::Disk,
            "/declarations" => Screen::Declarations,
            "/reply-rules" => Screen::Rules,
            "/localise" => Screen::Localise,
            "/translations" => Screen::Translations,
            "/source-updates" => Screen::Updates,
            "/external-changes" => Screen::Compare,
            "/translation" => Screen::Entry,
            _ => return Err("Unknown writer screen"),
        };
        let mut result = Self {
            screen,
            ..Self::default()
        };
        let mut seen = std::collections::BTreeSet::new();
        for (key, value) in url.query_pairs() {
            if !seen.insert(key.to_string()) || value.is_empty() {
                return Err("Invalid writer link parameter");
            }
            match key.as_ref() {
                "entry" => result.entry = Some(value.into_owned()),
                "project" => result.project = Some(value.into_owned()),
                "scene" => result.document = value.into_owned(),
                "beat" => result.beat = Some(value.into_owned()),
                "passage" => result.passage = Some(value.into_owned()),
                "catalogue" => result.catalogue = Some(value.into_owned()),
                "view" if value == "source" => result.source = true,
                "view" if value == "script" => {
                    result.view = Some(recite_config::WriterView::Script)
                }
                "view" if value == "map" => result.view = Some(recite_config::WriterView::Map),
                "q" => result.query = value.into_owned(),
                "filter" if value == "attention" => result.attention = true,
                "page" => result.page = value.parse().map_err(|_| "Invalid queue page")?,
                _ => return Err("Unknown writer link parameter"),
            }
        }
        if result.screen == Screen::Rules && (result.passage.is_none() || result.source) {
            return Err("A reply-rules link needs a passage and cannot be Source view");
        }
        if result.source && (result.beat.is_some() || result.passage.is_some()) {
            return Err("Conflicting writer views");
        }
        Ok(result)
    }
}
impl Routable for Location {
    const SITE_MAP: &'static [SiteMapSegment] = &[
        SiteMapSegment {
            segment_type: SegmentType::Static("disk-changes"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("rename"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("build"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("declarations"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("reply-rules"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("preview"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("translation"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("external-changes"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("source-updates"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("write"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("localise"),
            children: &[],
        },
        SiteMapSegment {
            segment_type: SegmentType::Static("translations"),
            children: &[],
        },
    ];
    fn render(&self, _: usize) -> Element {
        super::Shell.into_element()
    }
}

#[cfg(test)]
mod tests;
