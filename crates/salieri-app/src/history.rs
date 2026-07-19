use std::collections::VecDeque;

use salieri_core::Song;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SongPatch {
    before: Song,
    after: Song,
}

impl SongPatch {
    fn between(before: Song, after: Song) -> Option<Self> {
        (before != after).then_some(Self { before, after })
    }

    fn apply(&self, song: &mut Song) {
        *song = self.after.clone();
    }

    fn revert(&self, song: &mut Song) {
        *song = self.before.clone();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransactionSpec {
    label: String,
    merge_key: Option<String>,
}

impl TransactionSpec {
    pub(crate) fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            merge_key: None,
        }
    }

    pub(crate) fn merged(label: impl Into<String>, merge_key: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            merge_key: Some(merge_key.into()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct SongTransaction {
    before: Song,
    staged: Song,
}

impl SongTransaction {
    pub(crate) fn new(song: &Song) -> Self {
        Self {
            before: song.clone(),
            staged: song.clone(),
        }
    }

    pub(crate) fn song_mut(&mut self) -> &mut Song {
        &mut self.staged
    }

    pub(crate) fn nested<E>(
        &mut self,
        edit: impl FnOnce(&mut SongTransaction) -> Result<(), E>,
    ) -> Result<(), E> {
        let checkpoint = self.staged.clone();
        match edit(self) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.staged = checkpoint;
                Err(error)
            }
        }
    }

    fn finish(self) -> Option<SongPatch> {
        SongPatch::between(self.before, self.staged)
    }
}

#[derive(Debug)]
struct HistoryEntry {
    spec: TransactionSpec,
    patch: SongPatch,
}

#[derive(Debug)]
pub(crate) struct UndoHistory {
    undo: VecDeque<HistoryEntry>,
    redo: VecDeque<HistoryEntry>,
    limit: usize,
}

impl UndoHistory {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            limit: limit.max(1),
        }
    }

    pub(crate) fn commit(
        &mut self,
        song: &mut Song,
        transaction: SongTransaction,
        spec: TransactionSpec,
    ) -> bool {
        let Some(patch) = transaction.finish() else {
            return false;
        };
        patch.apply(song);
        self.record(spec, patch);
        true
    }

    fn record(&mut self, spec: TransactionSpec, patch: SongPatch) {
        self.redo.clear();
        let can_merge = spec.merge_key.is_some()
            && self
                .undo
                .back()
                .is_some_and(|entry| entry.spec.merge_key == spec.merge_key);

        if can_merge {
            let previous = self.undo.back_mut().expect("merge entry exists");
            previous.patch.after = patch.after;
            previous.spec.label = spec.label;
            if previous.patch.before == previous.patch.after {
                self.undo.pop_back();
            }
            return;
        }

        self.undo.push_back(HistoryEntry { spec, patch });
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    pub(crate) fn undo(&mut self, song: &mut Song) -> Option<String> {
        let entry = self.undo.pop_back()?;
        entry.patch.revert(song);
        let label = entry.spec.label.clone();
        self.redo.push_back(entry);
        Some(label)
    }

    pub(crate) fn redo(&mut self, song: &mut Song) -> Option<String> {
        let entry = self.redo.pop_back()?;
        entry.patch.apply(song);
        let label = entry.spec.label.clone();
        self.undo.push_back(entry);
        Some(label)
    }

    pub(crate) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    #[cfg(test)]
    pub(crate) fn undo_len(&self) -> usize {
        self.undo.len()
    }

    #[cfg(test)]
    pub(crate) fn redo_len(&self) -> usize {
        self.redo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit_bpm(history: &mut UndoHistory, song: &mut Song, bpm: u16, spec: TransactionSpec) {
        let mut transaction = SongTransaction::new(song);
        transaction.song_mut().transport.bpm = bpm;
        assert!(history.commit(song, transaction, spec));
    }

    #[test]
    fn patch_applies_and_reverts_a_complete_edit() {
        let mut song = Song::empty();
        let original = song.clone();
        let mut history = UndoHistory::new(10);

        commit_bpm(
            &mut history,
            &mut song,
            150,
            TransactionSpec::new("Set BPM"),
        );
        assert_eq!(song.transport.bpm, 150);
        assert_eq!(history.undo(&mut song), Some("Set BPM".to_string()));
        assert_eq!(song, original);
        assert_eq!(history.redo(&mut song), Some("Set BPM".to_string()));
        assert_eq!(song.transport.bpm, 150);
    }

    #[test]
    fn nested_failure_rolls_back_to_its_checkpoint() {
        let song = Song::empty();
        let mut transaction = SongTransaction::new(&song);
        transaction.song_mut().transport.bpm = 130;

        let result = transaction.nested(|nested| {
            nested.song_mut().transport.lines_per_beat = 8;
            Err::<(), _>("invalid nested edit")
        });

        assert_eq!(result, Err("invalid nested edit"));
        assert_eq!(transaction.song_mut().transport.bpm, 130);
        assert_eq!(
            transaction.song_mut().transport.lines_per_beat,
            song.transport.lines_per_beat
        );
    }

    #[test]
    fn merge_preserves_first_before_and_latest_after() {
        let mut song = Song::empty();
        let original_bpm = song.transport.bpm;
        let mut history = UndoHistory::new(10);
        for bpm in [121, 122, 123] {
            commit_bpm(
                &mut history,
                &mut song,
                bpm,
                TransactionSpec::merged("Adjust BPM", "transport.bpm"),
            );
        }

        assert_eq!(history.undo_len(), 1);
        history.undo(&mut song);
        assert_eq!(song.transport.bpm, original_bpm);
        history.redo(&mut song);
        assert_eq!(song.transport.bpm, 123);
    }

    #[test]
    fn bounded_history_drops_oldest_entries() {
        let mut song = Song::empty();
        let mut history = UndoHistory::new(2);
        for bpm in [121, 122, 123] {
            commit_bpm(
                &mut history,
                &mut song,
                bpm,
                TransactionSpec::new("Set BPM"),
            );
        }

        assert_eq!(history.undo_len(), 2);
        history.undo(&mut song);
        history.undo(&mut song);
        assert_eq!(song.transport.bpm, 121);
        assert!(history.undo(&mut song).is_none());
    }

    #[test]
    fn new_edit_invalidates_redo() {
        let mut song = Song::empty();
        let mut history = UndoHistory::new(10);
        commit_bpm(
            &mut history,
            &mut song,
            130,
            TransactionSpec::new("Set BPM"),
        );
        history.undo(&mut song);
        assert_eq!(history.redo_len(), 1);

        commit_bpm(
            &mut history,
            &mut song,
            140,
            TransactionSpec::new("Set BPM"),
        );
        assert_eq!(history.redo_len(), 0);
        assert!(history.redo(&mut song).is_none());
    }
}
