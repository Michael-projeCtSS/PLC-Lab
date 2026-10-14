//! Language-neutral anchors; graphical elements need no text coordinates.

use crate::identity::{DocumentId, SourceElementId};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SourceLanguage {
    St,
    Lad,
    Fbd,
    Stl,
}

/// One-based Unicode scalar columns, independent of editor encodings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePosition {
    line: u32,
    column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceError {
    ZeroPosition,
    ReversedRange,
    UnknownDocument,
    DuplicateDocument,
    LanguageMismatch,
    DuplicateElement,
}
impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ZeroPosition => "Source coordinates must be one-based.",
            Self::ReversedRange => "Source span end must not precede its start.",
            Self::UnknownDocument => "The source document is not registered.",
            Self::DuplicateDocument => "The source document identity is already registered.",
            Self::LanguageMismatch => "The source element language differs from its document.",
            Self::DuplicateElement => {
                "The source element identity is already registered in this document."
            }
        })
    }
}
impl std::error::Error for SourceError {}
impl SourcePosition {
    pub fn new(line: u32, column: u32) -> Result<Self, SourceError> {
        if line == 0 || column == 0 {
            return Err(SourceError::ZeroPosition);
        }
        Ok(Self { line, column })
    }
    pub fn line(self) -> u32 {
        self.line
    }
    pub fn column(self) -> u32 {
        self.column
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSpan {
    start: SourcePosition,
    end: SourcePosition,
}
impl TextSpan {
    pub fn new(start: SourcePosition, end: SourcePosition) -> Result<Self, SourceError> {
        if end < start {
            return Err(SourceError::ReversedRange);
        }
        Ok(Self { start, end })
    }
    pub fn start(self) -> SourcePosition {
        self.start
    }
    pub fn end(self) -> SourcePosition {
        self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    document_id: DocumentId,
    element_id: SourceElementId,
}
impl SourceRef {
    pub fn new(document_id: DocumentId, element_id: SourceElementId) -> Self {
        Self {
            document_id,
            element_id,
        }
    }
    pub fn document_id(self) -> DocumentId {
        self.document_id
    }
    pub fn element_id(self) -> SourceElementId {
        self.element_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocation {
    reference: SourceRef,
    language: SourceLanguage,
    text_span: Option<TextSpan>,
}
impl SourceLocation {
    pub fn new(
        reference: SourceRef,
        language: SourceLanguage,
        text_span: Option<TextSpan>,
    ) -> Self {
        Self {
            reference,
            language,
            text_span,
        }
    }
    pub fn reference(&self) -> SourceRef {
        self.reference
    }
    pub fn language(&self) -> SourceLanguage {
        self.language
    }
    pub fn text_span(&self) -> Option<TextSpan> {
        self.text_span
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceMap {
    documents: BTreeMap<DocumentId, SourceLanguage>,
    elements: BTreeMap<SourceRef, SourceLocation>,
}
impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register_document(
        &mut self,
        id: DocumentId,
        language: SourceLanguage,
    ) -> Result<(), SourceError> {
        if self.documents.contains_key(&id) {
            return Err(SourceError::DuplicateDocument);
        }
        self.documents.insert(id, language);
        Ok(())
    }
    pub fn register_element(&mut self, location: SourceLocation) -> Result<(), SourceError> {
        let language = self
            .documents
            .get(&location.reference.document_id)
            .ok_or(SourceError::UnknownDocument)?;
        if *language != location.language {
            return Err(SourceError::LanguageMismatch);
        }
        if self.elements.contains_key(&location.reference) {
            return Err(SourceError::DuplicateElement);
        }
        self.elements.insert(location.reference, location);
        Ok(())
    }
    pub fn get(&self, reference: SourceRef) -> Option<&SourceLocation> {
        self.elements.get(&reference)
    }
    pub fn document_language(&self, id: DocumentId) -> Option<SourceLanguage> {
        self.documents.get(&id).copied()
    }
    pub fn elements(&self) -> impl Iterator<Item = &SourceLocation> {
        self.elements.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reference(document: u32, element: u32) -> SourceRef {
        SourceRef::new(
            DocumentId::new(document).unwrap(),
            SourceElementId::new(element).unwrap(),
        )
    }
    #[test]
    fn text_positions_spans_points_and_multiline_ranges_are_checked() {
        assert_eq!(SourcePosition::new(0, 1), Err(SourceError::ZeroPosition));
        assert_eq!(SourcePosition::new(1, 0), Err(SourceError::ZeroPosition));
        let start = SourcePosition::new(2, 5).unwrap();
        assert_eq!(
            TextSpan::new(start, SourcePosition::new(1, 9).unwrap()),
            Err(SourceError::ReversedRange)
        );
        assert_eq!(
            TextSpan::new(start, SourcePosition::new(2, 4).unwrap()),
            Err(SourceError::ReversedRange)
        );
        assert_eq!(TextSpan::new(start, start).unwrap().start(), start);
        let end = SourcePosition::new(u32::MAX, u32::MAX).unwrap();
        assert_eq!(TextSpan::new(start, end).unwrap().end(), end);
        assert_eq!((end.line(), end.column()), (u32::MAX, u32::MAX));
    }
    #[test]
    fn all_languages_use_the_same_anchor_without_requiring_text() {
        let mut map = SourceMap::new();
        for (index, language) in [
            SourceLanguage::St,
            SourceLanguage::Lad,
            SourceLanguage::Fbd,
            SourceLanguage::Stl,
        ]
        .into_iter()
        .enumerate()
        {
            let source = reference(index as u32 + 1, 1);
            map.register_document(source.document_id(), language)
                .unwrap();
            let span = matches!(language, SourceLanguage::St | SourceLanguage::Stl).then(|| {
                TextSpan::new(
                    SourcePosition::new(1, 1).unwrap(),
                    SourcePosition::new(1, 5).unwrap(),
                )
                .unwrap()
            });
            let location = SourceLocation::new(source, language, span);
            map.register_element(location.clone()).unwrap();
            assert_eq!(map.get(source), Some(&location));
            assert_eq!(map.get(source).unwrap().language(), language);
            assert_eq!(map.get(source).unwrap().text_span(), span);
        }
        assert_eq!(map.elements().count(), 4);
    }
    #[test]
    fn source_registry_rejects_dangling_duplicate_and_mismatched_metadata_atomically() {
        let mut map = SourceMap::new();
        let source = reference(1, 1);
        let location = SourceLocation::new(source, SourceLanguage::Lad, None);
        assert_eq!(
            map.register_element(location.clone()),
            Err(SourceError::UnknownDocument)
        );
        map.register_document(source.document_id(), SourceLanguage::Lad)
            .unwrap();
        assert_eq!(
            map.register_document(source.document_id(), SourceLanguage::St),
            Err(SourceError::DuplicateDocument)
        );
        assert_eq!(
            map.register_element(SourceLocation::new(source, SourceLanguage::St, None)),
            Err(SourceError::LanguageMismatch)
        );
        map.register_element(location.clone()).unwrap();
        assert_eq!(
            map.register_element(location.clone()),
            Err(SourceError::DuplicateElement)
        );
        assert_eq!(map.get(source), Some(&location));
        assert_eq!(
            map.document_language(source.document_id()),
            Some(SourceLanguage::Lad)
        );
        assert!(map.get(reference(1, 2)).is_none());
        assert!(map.get(reference(2, 1)).is_none());
    }
}
