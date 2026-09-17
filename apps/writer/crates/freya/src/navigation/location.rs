//! Linkable workspace locations; no edits or manuscript content in URLs.
use freya::{prelude::*, router::*};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Screen {
    #[default]
    Write,
    Localise,
    Translations,
    Updates,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Location {
    pub screen: Screen,
    pub document: String,
    pub project: Option<String>,
    pub beat: Option<String>,
    pub passage: Option<String>,
    pub source: bool,
    pub catalogue: Option<String>,
    pub query: String,
    pub attention: bool,
    pub page: usize,
}
impl Location {
    pub fn same_place(&self, other: &Self) -> bool {
        self.project == other.project
            && self.screen == other.screen
            && self.document == other.document
            && self.beat == other.beat
            && self.source == other.source
            && self.catalogue == other.catalogue
    }
}
impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = match self.screen {
            Screen::Write => "/write",
            Screen::Localise => "/localise",
            Screen::Translations => "/translations",
            Screen::Updates => "/source-updates",
        };
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        if let Some(project) = &self.project {
            query.append_pair("project", project);
        }
        if !self.document.is_empty() {
            query.append_pair("scene", &self.document);
        }
        if let Some(beat) = &self.beat {
            query.append_pair("beat", beat);
        }
        if let Some(passage) = &self.passage {
            query.append_pair("passage", passage);
        }
        if self.source {
            query.append_pair("view", "source");
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
            "/localise" => Screen::Localise,
            "/translations" => Screen::Translations,
            "/source-updates" => Screen::Updates,
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
                "project" => result.project = Some(value.into_owned()),
                "scene" => result.document = value.into_owned(),
                "beat" => result.beat = Some(value.into_owned()),
                "passage" => result.passage = Some(value.into_owned()),
                "catalogue" => result.catalogue = Some(value.into_owned()),
                "view" if value == "source" => result.source = true,
                "q" => result.query = value.into_owned(),
                "filter" if value == "attention" => result.attention = true,
                "page" => result.page = value.parse().map_err(|_| "Invalid queue page")?,
                _ => return Err("Unknown writer link parameter"),
            }
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
