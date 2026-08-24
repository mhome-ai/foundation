pub const THREAD_LIST_TARGET: &str = "/chat/thread/list";
pub const THREAD_CREATE_TARGET: &str = "/chat/thread/create";
pub const THREAD_ARCHIVE_TARGET: &str = "/chat/thread/archive";
pub const THREAD_ROTATE_TARGET: &str = "/chat/thread/rotate";
pub const THREAD_LOAD_TARGET: &str = "/chat/thread/load";
pub const MESSAGE_ENQUEUE_TARGET: &str = "/chat/message/enqueue";
pub const QUEUE_REORDER_TARGET: &str = "/chat/queue/reorder";
pub const REQUEST_CANCEL_TARGET: &str = "/chat/request/cancel";
pub const INTERACTION_SUBMIT_TARGET: &str = "/chat/interaction/submit";
pub const CHAT_EVENT_TARGET: &str = "/chat/event";
pub const CHAT_DEBUG_TARGET: &str = "/chat/debug";

pub fn is_conversation_request_target(target: &str) -> bool {
    matches!(
        target,
        THREAD_LIST_TARGET
            | THREAD_CREATE_TARGET
            | THREAD_ARCHIVE_TARGET
            | THREAD_ROTATE_TARGET
            | THREAD_LOAD_TARGET
            | MESSAGE_ENQUEUE_TARGET
            | QUEUE_REORDER_TARGET
            | REQUEST_CANCEL_TARGET
            | INTERACTION_SUBMIT_TARGET
    )
}
