
use smoltcp::wire::EthernetAddress;

use crate::{nics::{NicAllocator, NicsMut, Nics}, node::{Mailbox, Node, NodeError}};

const ETH0: EthernetAddress = EthernetAddress([0, 0, 0, 0, 0, 0]);

struct BasicNode {
    add_nic: bool,
}

#[async_trait::async_trait]
impl Node for BasicNode {

    fn hardware(&self, nics: &mut NicAllocator) {
        if self.add_nic {
            nics.add(ETH0, None);
        }
    }

    fn startup(&mut self, nics: &mut NicsMut<'_>) {
        todo!()
    }

    async fn process(&mut self, _: &mut Mailbox, _: &Nics<'_>) -> Result<(), NodeError> {
        Ok(())
    }
}