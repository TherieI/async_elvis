# Elvis's Async Runtime

*Gabriel Rosas*



## Purpose

The current Elvis runtime runs nodes synchronously; they are scheduled, then polled, currently without concurrency or multithreading. The runtime will call poll for nodes appropriately, providing the user's implementation with all incoming messages to the node at the beginning, and receiving any outgoing messages from the node upon return. However, within poll, a node cannot currently request a future message or send any outgoing messages before returning. This means users have to implement a cache for messages they receive when they desire parsing their contents at future times, using future information.

```rust
// Current implementation
pub trait Node {
    fn poll(&mut self, time: Instant, incoming: Vec<IncomingMsg>) -> Vec<OutgoingMsg>;
    
    // ...
}
```

The introduction of an asynchronous way to send and receive messages inside poll attempts to solve this issue. Instead of `Node::poll`'s former signature, we would encapsulate both the incoming and outgoing messages within a `Mail` struct. Users would be capable of calling `mail.receive()` and `mail.send(...)` to communicate with other nodes within the scope of their poll function.

```rust
pub trait Node {
    async fn poll(&mut self, time: Instant, mail: Mail);
    
    // ...
}

impl Node for RouterNode {
    async fn poll(&mut self, time: Instant, mail: Mail) {
        let next_message: IncomingMsg = mail.receive().await;
        
        // Perhaps a situation arises where the node needs to make an arp request
        // before parsing `next_message`...
        
        let arp_request = /* ... */ ;
        mail.send(arp_request);
        // Assuming the arp response is the next message in the inbox...
        let arp_response = mail.receive().await;
        
        // Continue with `next_message`...
    }
}
```



## Architecture

- `Mailbox` - a cache to store a node's messages
  - `IncomingMsg`
  - `OutgoingMsg`

I think there needs to be some way to tie a future's `context` to Mailbox.

```rust
pub struct Mail {
    messages: VecDeque<IncomingMsg>
}

impl Mail {
    pub(crate) fn new();
    pub(crate) fn notify();
    
    pub fn send(&mut self, msg: IncomingMsg);
    pub fn receive(&mut self) -> NextMessage<'_>;
}
```



How will the waker function in the context of elvis?

> The executor keeps track of messages sent to each `Mail`, and will call `Waker::wake` on any `Mail` receiving new messages, if there is an associated waker

How does the executor become aware that `Waker::wake` has been called if it is not the one calling it?

> The typical life of a `Waker` is that it is constructed by an executor, wrapped in a [`Context`](https://doc.rust-lang.org/beta/core/task/struct.Context.html), then passed to [`Future::poll()`](https://doc.rust-lang.org/beta/core/future/trait.Future.html#tymethod.poll). Then, if the future chooses to return [`Poll::Pending`](https://doc.rust-lang.org/beta/core/task/enum.Poll.html#variant.Pending), it must also store the waker somehow and call [`Waker::wake()`](https://doc.rust-lang.org/beta/core/task/struct.Waker.html#method.wake) when the future should be polled again.

```rust
impl Future for NextMessage<'_> {
    type Output = IncomingMsg;
    
    // NON-BLOCKING
    fn poll(mut self: Pin<&mut self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(next) = self.mail.messages.pop_front() {
            Poll::Ready(next)
        } else {
            // From docs:
            // When a future is not yet ready, `poll` returns `Poll::Pending` and stores
            // a clone of the `Waker` copied from the current `Context`. This `Waker` is
            // then woken once the future can make progress.
            
            // How should NextMessage store a clone of the waker? Must it absolutely have to?
            Poll::Pending
        }
    }
}
```



### `std::alloc::task::Wake` vs `RawWakeVTable`

Unsafe code - vtable construction

 ```rust
unsafe fn clone(_: *const ()) -> RawWaker;
unsafe fn wake(_: *const ());
unsafe fn wake_by_ref(_: *const ());
unsafe fn drop(_: *const ());
 ```



## Unrelated Issues

- The node `Index` system
  - Nodes are stored in an array of nodes `&mut [Node]`, passed to the `run_sim` function.
  - An equally sized array of `Inboxes` are created for each respective node.
  - Messages are sent from node to node via the `OutgoingMsg` struct, which specifies a recipient as an index in the nodes array.
  - Theoretically, there are no restrictions as to what neighboring nodes a node can send messages to.
  - There needs to be some limitations as to who nodes can send to, in real life represented by ethernet and wireless connections.
- We need a way to link nodes (via "ethernet") so the simulation isn't a full mesh. `nics`?
  - How



## Design Guide



### Node refactor | Part 1

This first iteration of the node trait serves to discern a definitive framework for linking nodes together via ethernet-like connections. There will be a method to set up the hardware aspect of a node, run once; a method to link with the hardware of other nodes, run once; and an async method for interactions when the simulation is running. The API definitions are subject to change.

```rust
pub trait Node {
    /// Runs once at the beginning of the simulation.
    /// Defines NICs and hardware properties.
    fn setup(&mut self, nics: &mut Vec<Nic>);
    
    /// Runs once at the beginning of the simulation, after all nodes have ran setup.
    /// Form neighborships between other nodes.
    fn on_startup(&mut self, nics: &Vec<Nic>);
    
    /// Called when a node receives message(s) in it's mailbox.
    /// Process messages and tasks, able to determine from what neighbor/interface 
    /// the message was received on.
    async fn process(&mut self, mail: &mut Mailbox, neighbors: &Vec<Nic>);
}

/// [EXAMPLE NODE IMPLEMENTATION]
impl Node for SomeExampleNode {
    fn setup(&mut self, nics: &mut Vec<Nic>) {
        // Setup NIC interfaces
        nics.add(
            Nic::new(
            	Mac::random(), 
            	IfaceCapabilities::GigabitEthernet,
            )
        );
        nics.add(
            Nic::new(
            	Mac::from([1, 2, 3, 4, 5, 6]),
            	IfaceCapabilities::FastEthernet,
            )
        );
    }
    
    fn on_startup(&mut self, nics: &Vec<Nic>) {
        // Link the first nic with the neighbor, 02:03:04:05:06:07 
    	nics[0].link(Mac::from([2, 3, 4, 5, 6, 7]));
        
        // Send an arp request to the neighbor
        mailbox.send(nics[0], Arp::request());
    }
    
    async fn process(&mut self, mail: &mut Mailbox, nics: &Vec<Nic>) {
        // Receive arp response
        mailbox.receive(nics[0]).await;
    }
}
```



### Network Interface Card (NIC)

Currently, `Nic` is an imaginary struct in the realm of the simulation. Let's define something more concrete.

```rust
pub type NicId 		= u64;
pub type NicGroup 	= u64;
pub type LinkId 	= u64;

pub struct Nic {
    pub(crate) id: NicId,
    
    /// The node the Nic is accociated with.
    pub(crate) group: NicGroup,

    pub(crate) mac: EthernetAddress,
    pub(crate) latency: Option<u64>,

    // A link id will be generated when two nodes connect. The value will be shared across both NICs.
    pub(crate) link_id: Option<LinkId>,
}
```

It is intended for a set of Nics on a device to be initialized within the `Node` trait. The entirety of the simulation's Nics should be stored in an array - `Vec<Nic>` - with the Nic ID (`NicId`) representing the index of the Nic in the Nics array. Nics are bundled in groups (`NicGroup`), each group corresponding with a singular node. This means we can easily generate mutable slices of Nics with the `[T]::chunk_by_mut` function, *so long as each node has at LEAST one NIC*. For example:

```rust
let mut nodes: &mut [&mut dyn Node] = /* ... retrieve Nodes ... */ ;
let mut all_nics: Vec<Nic> = /* ... retrieve all nics ... */ ;
let mut nics_group: Vec<&mut [Nic]> = all_nics.chunk_by_mut(|l, r| l.group == r.group).collect();
```

With this definition, we can assert that the nics for `nodes[0]` will be bundled in `nics_group[0]`, the nics for `nodes[1]` in `nics_group[1]`, ... etc.

Naturally, we need a way to ascertain links from Nic-to-Nic in the simulation.

```rust
/// Storing link information
pub(crate) struct Topology {
    next_id: LinkId,
    links: HashMap<LinkId, (NicId, NicId)>,
}
```



### Node refactor | Part 2

The previous node was a fine start, but after implementing the `Nic` struct, there are a few extra things I want to take into consideration with this new design:

- Intuitiveness (better naming/API)
- Identification (for visualization).
- Scheduled events / timers.

```rust
#[async_trait]
pub trait Node {
    /// Identification of the node. 
    /// Nodes default to "Node" as a name.
    fn name(&self) -> &str {
        "Node"
    }

    /// Add Network Interface Cards and hardware functionality to the node.
    /// This function will run once before `startup` is called.
    fn hardware(&self, nics: &mut NicAllocator);

    /// Connect to other devices.
    fn startup(&mut self, nics: &mut Nics);

    /// Called when the node's `Mailbox` has incoming messages. 
    async fn process(&mut self, mail: &mut Mailbox, nics: &Nics) -> Result<(), NodeError>;
}
```

There are now new structs we have to figure out to abstract `Vec<Nic>`, such as `Nics` and `NicAllocator`, but everything is slightly more coherent. However, there is still a problem in scheduled events / timers. Should `Mailbox` itself have a way to generate clock-like futures? Futures that poll every set amount of time?



### Node refactor | Part 3

```rust
#[async_trait]
pub trait Node {
    /// Identification of the node.
    /// Nodes default to "Node" as a name.
    fn name(&self) -> &str {
        "Node"
    }

    /// Add Network Interface Cards and hardware functionality to the node.
    /// This function will run once before `startup` is called.
    fn hardware(&self, nics: &mut NicAllocator);

    /// Connect to other devices.
    fn startup(&mut self, nics: &mut NicsMut<'_>);

    /// Called when the node's `Mailbox` has incoming messages.
    async fn process(&mut self, mail: &mut Mailbox, nics: &Nics<'_>) -> Result<(), NodeError>;
}
```



### Simulation

- Generate hardware for nodes.
- Link nodes together.


### Troubles

I'm beginning to think that linking nodes may be more nuanced than the system I came up with can handle. Say, for example, we have a router. The router may generate X number of Nics, but the user may not be aware of those Ethernet addresses. If that is the case, how can the router link up with the sim? (potentially through name?).

Ideal linking API:
`nics[i].link(&Mac);`

Potential `Node` function:
```rust
/// Schedule the node to run after every specified duration (SIM TIME).
async fn clock(&mut self, _: &mut Mailbox, _: &Nics<'_>) -> Option<u128> {
    None
}
```

