// Prose findings and their fixes in the native Writer.
macro_rules! writer_prose_message_ids {
    () => {
        MsgId::WriterProseReplace
        | MsgId::WriterProseAddWord
        | MsgId::WriterProseApplied
        | MsgId::WriterProseStale
        | MsgId::WriterProseDraftPending
        | MsgId::WriterProseBusy
        | MsgId::WriterProseWordAdded
        | MsgId::WriterProseWordListed
        | MsgId::WriterProseDictionaryFailed
        | MsgId::WriterProseSpelling
        | MsgId::WriterProseNote
        | MsgId::WriterProseCount
    };
}
