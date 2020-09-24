//! The entity system.
//!
//! This system is responsible in loading, streaming and watching assets, also known as _entities_. Entities are
//! independent objects identified by a unique identifier.

pub mod mesh;

use crate::{
  runtime::RuntimeMsg,
  system::{resource::ResourceManager, system_init, Addr, MsgQueue, Subscriber, System, SystemUID},
};
use colored::Colorize as _;
use mesh::Mesh;
use std::{
  ffi::OsStr,
  fs::read_dir,
  path::{Path, PathBuf},
  sync::Arc,
  thread,
};

/// All possible entities.
#[derive(Debug)]
pub enum Entity {
  /// A [`Mesh`].
  Mesh(Mesh),
}

#[derive(Clone, Debug)]
pub enum EntityMsg {
  /// Kill message.
  Kill,
}

#[derive(Clone, Debug)]
pub enum EntityEvent {
  HelloWorld,
}

/// The [`Entity`] system.
pub struct EntitySystem {
  uid: SystemUID,
  runtime_addr: Addr<RuntimeMsg>,
  /// Directory where all scarce resources this entity system knows about live in.
  root_dir: PathBuf,
  resources: ResourceManager<Entity>,
  addr: Addr<EntityMsg>,
  msg_queue: MsgQueue<EntityMsg>,
  subscribers: Vec<Box<dyn Subscriber<EntityEvent>>>,
}

impl EntitySystem {
  /// Create a new [`EntitySystem`].
  pub fn new(runtime_addr: Addr<RuntimeMsg>, uid: SystemUID, root_dir: impl AsRef<Path>) -> Self {
    let (addr, msg_queue) = system_init();

    Self {
      uid,
      runtime_addr,
      root_dir: root_dir.as_ref().to_owned(),
      resources: ResourceManager::new(),
      addr,
      msg_queue,
      subscribers: Vec::new(),
    }
  }

  /// Start the system.
  ///
  /// This method will first tries to load all the resources it can from `root_dir`, then will stay in an idle mode where it will:
  ///
  /// - Wait for system messages.
  /// - Wait for filesystem notifications.
  pub fn start(mut self) {
    let root_dir = self.root_dir.clone(); // TODO: check how we can remove the clone
    self.traverse_directory(&root_dir);

    // main loop
    loop {
      match self.msg_queue.recv() {
        Some(EntityMsg::Kill) | None => {
          self
            .runtime_addr
            .send_msg(RuntimeMsg::SystemExit(self.uid))
            .unwrap();
          break;
        }
      }
    }
  }

  fn traverse_directory(&mut self, path: &Path) {
    log::debug!(
      "traversing {}",
      path.display().to_string().purple().italic(),
    );

    for dir_entry in read_dir(path).unwrap() {
      let file = dir_entry.unwrap();
      let path = file.path();

      if path.is_dir() {
        // recursively traverse this repository
        self.traverse_directory(&path);
      } else if path.is_file() {
        // invoke a handler for this file
        log::info!(
          "found resource file {}",
          path.display().to_string().purple().italic(),
        );

        // extract the extension
        match path.extension().and_then(OsStr::to_str) {
          Some(ext) => {
            self.extension_based_dispatch(ext, &path);
          }

          None => {
            log::warn!(
              "resource {} doesn’t have a path extension; ignoring",
              path.display().to_string().purple().italic(),
            );
          }
        }
      }
    }
  }

  /// Dispatch entity loading based on the extension of a file.
  fn extension_based_dispatch(&mut self, ext: &str, path: &Path) {
    match ext {
      "obj" => self.load_obj(path),
      _ => log::warn!(
        "unknown extension {} for path {}",
        ext.blue().italic(),
        path.display().to_string().purple().italic(),
      ),
    }
  }

  /// Load .obj files.
  fn load_obj(&mut self, path: &Path) {
    match Mesh::load_from_path(path) {
      Ok(mesh) => {
        let path_name = path.display().to_string();
        let path = path_name.purple().italic();
        log::info!("{} {}", "loaded".green().bold(), path);

        let h = self.resources.wrap(Entity::Mesh(mesh), path_name);
        log::debug!("assigned {} handle {}", path, h.to_string().green().bold());
      }

      Err(err) => {
        log::error!(
          "cannot load OBJ {}: {}",
          path.display().to_string().purple().italic(),
          err,
        );
      }
    }
  }
}

impl System<EntityEvent> for EntitySystem {
  type Addr = Addr<EntityMsg>;

  fn system_addr(&self) -> Addr<EntityMsg> {
    self.addr.clone()
  }

  fn startup(self) {
    // move into a thread for greater good
    let _ = thread::spawn(move || {
      self.start();
    });
  }

  fn subscribe(&mut self, subscriber: impl Subscriber<EntityEvent> + 'static) {
    self.subscribers.push(Box::new(subscriber));
  }

  fn publish(&self, event: EntityEvent) {
    for sub in &self.subscribers {
      sub.recv_msg(event.clone());
    }
  }
}