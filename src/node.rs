use async_trait::async_trait;

use crate::nics::*;

#[macro_export]
macro_rules! nodes {
    ( $( $x:expr ),* ) => {
        &mut [$( &mut $x, )*]
    }
}

pub struct IncomingMsg {
    from: NicId,
    data: Vec<u8>,
}

impl IncomingMsg {}

pub struct OutgoingMsg {
    to: NicId,
    data: Vec<u8>,
}

pub struct Mailbox {
    incoming: Vec<IncomingMsg>,
}

pub enum NodeError {}

#[async_trait]
pub trait Node {
    /// Identification of the node.
    /// Nodes default to "Node" as a name.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Add Network Interface Cards and hardware functionality to the node.
    /// This function will run once before `startup` is called.
    fn hardware(&self, nics: &mut NicAllocator);

    /// Connect to other devices.
    fn startup(&mut self, nics: &mut NicsMut<'_>);

    /// Called when the node's `Mailbox` has incoming messages.
    async fn process(&mut self, mail: &mut Mailbox, nics: &Nics<'_>) -> Result<(), NodeError>;
}
