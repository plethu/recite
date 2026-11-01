use super::{Part, PreviewTrace, Record, Section};

pub const RECORDS: usize = 8;
pub const FIELDS: usize = 6;

#[derive(Clone, Copy, Default)]
pub struct Browser {
    pub open: bool,
    pub section: Section,
    pub place: Place,
}
#[derive(Clone, Copy)]
pub enum Place {
    List { page: usize },
    Record { index: usize, page: usize },
    Value(ValuePosition),
}
#[derive(Clone, Copy, Default)]
pub struct ValuePosition {
    pub record: usize,
    pub field: usize,
    pub part: usize,
}
impl Default for Place {
    fn default() -> Self {
        Self::List { page: 0 }
    }
}
pub enum Projection {
    List(ListPage),
    Record(RecordPage),
    Value(ValuePage),
}
pub struct ListPage {
    pub section: Section,
    pub total: usize,
    pub page: usize,
    pub pages: usize,
    pub rows: Vec<Row>,
}
pub struct Row {
    pub index: usize,
    pub summary: String,
}
pub struct RecordPage {
    pub index: usize,
    pub summary: String,
    pub page: usize,
    pub pages: usize,
    pub fields: Vec<FieldPreview>,
}
pub struct FieldPreview {
    pub index: usize,
    pub name: String,
    pub value: String,
    pub truncated: bool,
}
pub struct ValuePage {
    pub position: ValuePosition,
    pub summary: String,
    pub value: Part,
}
impl Browser {
    pub fn restart(&mut self) {
        self.place = Place::default();
    }
    pub fn select(&mut self, section: Section) {
        self.section = section;
        self.restart();
    }
    pub fn project(&self, trace: &PreviewTrace) -> Projection {
        match self.place {
            Place::List { page } => Projection::List(self.list(trace, page)),
            Place::Record { index, page } => self
                .record(trace, index, page)
                .map_or_else(|| Projection::List(self.list(trace, 0)), Projection::Record),
            Place::Value(position) => self
                .value(trace, position)
                .map_or_else(|| Projection::List(self.list(trace, 0)), Projection::Value),
        }
    }
    fn entry<'a>(&self, trace: &'a PreviewTrace, index: usize) -> Option<(usize, Record<'a>)> {
        let index = index.min(self.section.len(trace).saturating_sub(1));
        self.section
            .record(trace, index)
            .map(|entry| (index, entry))
    }
    fn list(&self, trace: &PreviewTrace, requested: usize) -> ListPage {
        let total = self.section.len(trace);
        let pages = total.div_ceil(RECORDS).max(1);
        let page = requested.min(pages - 1);
        let rows = (page * RECORDS..total.min((page + 1) * RECORDS))
            .filter_map(|index| {
                self.section.record(trace, index).map(|record| Row {
                    index,
                    summary: record.summary(),
                })
            })
            .collect();
        ListPage {
            section: self.section,
            total,
            page,
            pages,
            rows,
        }
    }
    fn record(&self, trace: &PreviewTrace, index: usize, requested: usize) -> Option<RecordPage> {
        let (index, entry) = self.entry(trace, index)?;
        let count = entry.field_count();
        let pages = count.div_ceil(FIELDS).max(1);
        let page = requested.min(pages - 1);
        let fields = (page * FIELDS..count.min((page + 1) * FIELDS))
            .filter_map(|index| {
                let field = entry.field(index)?;
                let (value, truncated) = field.excerpt();
                Some(FieldPreview {
                    index,
                    name: field.name.into_owned(),
                    value,
                    truncated,
                })
            })
            .collect();
        Some(RecordPage {
            index,
            summary: entry.summary(),
            page,
            pages,
            fields,
        })
    }
    fn value(&self, trace: &PreviewTrace, requested: ValuePosition) -> Option<ValuePage> {
        let (record, entry) = self.entry(trace, requested.record)?;
        let field = requested.field.min(entry.field_count().saturating_sub(1));
        let value = entry.field(field)?.part(requested.part);
        Some(ValuePage {
            position: ValuePosition {
                record,
                field,
                part: value.part,
            },
            summary: entry.summary(),
            value,
        })
    }
}
