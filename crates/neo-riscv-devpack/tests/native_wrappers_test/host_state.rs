use super::Invocation;

#[derive(Default)]
pub(super) struct HostState {
    pub(super) invocations: Vec<Invocation>,
    pub(super) response: Vec<u8>,
}
