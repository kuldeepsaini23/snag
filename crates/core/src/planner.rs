use crate::model::{AppState, ItemId, QueueId, Status};
use crate::schedule::Now;
use std::collections::{HashMap, HashSet};

/// Whether a queue may run right now.
pub fn queue_active(state: &AppState, queue: QueueId, now: Now) -> bool {
    match state.queue(queue) {
        Some(q) => q.schedule.as_ref().is_none_or(|s| s.is_active(now)),
        None => false,
    }
}

/// Queued items to start now: FIFO, only from active queues with room, up to the
/// global `max_concurrent`.
pub fn pick_next(state: &AppState, running: &HashSet<ItemId>, now: Now) -> Vec<ItemId> {
    let mut free = state.settings.max_concurrent.saturating_sub(running.len());
    let mut per_queue: HashMap<QueueId, usize> = HashMap::new();
    for item in state.items.iter().filter(|i| running.contains(&i.id)) {
        *per_queue.entry(item.queue).or_default() += 1;
    }
    let mut picked = Vec::new();
    for item in &state.items {
        if free == 0 {
            break;
        }
        if item.status != Status::Queued || running.contains(&item.id) || !queue_active(state, item.queue, now) {
            continue;
        }
        let Some(queue) = state.queue(item.queue) else { continue };
        let count = per_queue.entry(queue.id).or_default();
        if *count >= queue.max_concurrent {
            continue;
        }
        *count += 1;
        free -= 1;
        picked.push(item.id);
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::Category;
    use crate::model::{Item, Queue};
    use crate::schedule::Schedule;

    fn item(id: u64, queue: QueueId, status: Status) -> Item {
        Item {
            id: ItemId(id),
            url: format!("http://x/{id}"),
            name: format!("{id}.bin"),
            category: Category::Other,
            status,
            dest: None,
            downloaded: 0,
            total: None,
            speed_bps: 0,
            queue,
            added: 0,
            kind: Default::default(),
            referrer: None,
            work_dir: None,
        }
    }

    fn state(max: usize, items: Vec<Item>) -> AppState {
        let mut s = AppState::default();
        s.settings.max_concurrent = max;
        s.items = items;
        s
    }

    const NOON: Now = Now { weekday: 0, minute: 720 };

    #[test]
    fn planner_respects_global_limit() {
        let s = state(2, (1..=4).map(|i| item(i, 0, Status::Queued)).collect());
        let running: HashSet<_> = [ItemId(1)].into();
        assert_eq!(pick_next(&s, &running, NOON), vec![ItemId(2)]);
    }

    #[test]
    fn planner_respects_queue_limit() {
        let mut s = state(5, vec![item(1, 1, Status::Running), item(2, 1, Status::Queued), item(3, 0, Status::Queued)]);
        s.queues.push(Queue { id: 1, name: "Night".into(), max_concurrent: 1, schedule: None });
        let running: HashSet<_> = [ItemId(1)].into();
        assert_eq!(pick_next(&s, &running, NOON), vec![ItemId(3)]);
    }

    #[test]
    fn planner_skips_inactive_queue() {
        let mut s = state(5, vec![item(1, 1, Status::Queued), item(2, 0, Status::Queued)]);
        s.queues.push(Queue { id: 1, name: "Night".into(), max_concurrent: 5, schedule: Some(Schedule { start: 23 * 60, stop: Some(7 * 60), days: [true; 7] }) });
        assert_eq!(pick_next(&s, &HashSet::new(), NOON), vec![ItemId(2)]);
        assert_eq!(pick_next(&s, &HashSet::new(), Now { weekday: 0, minute: 23 * 60 + 5 }), vec![ItemId(1), ItemId(2)]);
    }

    #[test]
    fn planner_is_fifo_and_skips_non_queued() {
        let s = state(10, vec![item(1, 0, Status::Paused), item(2, 0, Status::Queued), item(3, 0, Status::Done), item(4, 0, Status::Queued)]);
        assert_eq!(pick_next(&s, &HashSet::new(), NOON), vec![ItemId(2), ItemId(4)]);
    }
}
