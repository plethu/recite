//! Freya router adapter for the shared, linkable product location.
use freya::{prelude::*, router::*};
pub(crate) use recite_writer_session::location::{Location, Screen};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Route(pub Location);
impl std::fmt::Display for Route {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
impl std::str::FromStr for Route {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}
impl Routable for Route {
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
