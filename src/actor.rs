use tokio::sync::{oneshot, mpsc};

struct Actor {
    receiver: mpsc::Receiver<ActorMessage>,
    next_id: u64,
}

enum ActorMessage {
    GetUniqueId {
        respond_to: oneshot::Sender<u64>,
    }
}

impl Actor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        Actor {
            receiver,
            next_id: 0,
        }
    }

    fn handle_message(&mut self, message: ActorMessage) {
        match message {
            ActorMessage::GetUniqueId { respond_to } => {
                self.next_id += 1;
                let _ = respond_to.send(self.next_id);
            },
        }
    }
}

async fn run_my_actor(mut actor: Actor) {
    while let Some(message) = actor.receiver.recv().await {
        actor.handle_message(message);
    }
}

#[derive(Clone)]
pub struct ActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

impl ActorHandle {
    pub fn new(_sender: mpsc::Sender<ActorMessage>) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let actor = Actor::new(receiver);
        tokio::spawn(run_my_actor(actor));
        Self { sender }
    }

    pub async fn get_unique_id(&self) -> u64 {
        let (send, recv) = oneshot::channel();
        let message = ActorMessage::GetUniqueId { respond_to: send };
        let _ = self.sender.send(message).await;
        recv.await.expect("Actor task has been killed")
    }
    
}