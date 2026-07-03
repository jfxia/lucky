
/// A source location span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub file_id: FileId,
}

impl Span {
    pub const DUMMY: Span = Span { start: 0, end: 0, file_id: FileId(0) };

    pub fn new(start: usize, end: usize, file_id: FileId) -> Self {
        Self { start, end, file_id }
    }

    pub fn merge(self, other: Span) -> Span {
        assert_eq!(self.file_id, other.file_id);
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            file_id: self.file_id,
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// Opaque file identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);
