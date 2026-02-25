use dats::{context::DatContext, formats::events::EventBlock};

/// Enrich event blocks with item/cost annotations. Currently disabled (too many false positives).
pub fn enrich_event_blocks(
    _blocks: &mut [EventBlock],
    _dat_context: &DatContext,
    _zone_id: u16,
) {
}
