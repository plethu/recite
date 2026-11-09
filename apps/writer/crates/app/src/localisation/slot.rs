//! A catalogue has one mutable owner; exclusive work leaves a small read-only presentation.
use recite_compiler::authoring::{CancellationToken, WorkControl};
use recite_core::po::{PoDocument, PoEntryId};
use recite_writer_session::localisation::catalogue::{Catalogue, Draft};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone)]
pub struct Identity(Arc<()>);
impl PartialEq for Identity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for Identity {}
pub struct Owner {
    pub(crate) identity: Identity,
    pub catalogue: Catalogue,
    access: Access,
}
enum Access {
    Editable,
    ReopenRequired,
}
impl Owner {
    pub fn new(catalogue: Catalogue) -> Self {
        Self {
            identity: Identity(Arc::new(())),
            catalogue,
            access: Access::Editable,
        }
    }
    pub(crate) fn renew_identity(&mut self) {
        self.identity = Identity(Arc::new(()));
    }
    pub(crate) fn quarantine(&mut self) {
        self.access = Access::ReopenRequired;
    }
    pub(crate) fn flush_recovery(&mut self) -> Result<(), String> {
        match self.access {
            Access::Editable => self.catalogue.flush_recovery(),
            Access::ReopenRequired => self.catalogue.flush_admitted_recovery(),
        }
    }
}
struct Claim {
    cancel: CancellationToken,
}
#[derive(Clone)]
pub struct Ticket(Arc<Claim>);
impl Ticket {
    pub fn cancel(&self) {
        self.0.cancel.interrupt();
    }
}
impl WorkControl for Ticket {
    fn checkpoint(&self) -> Result<(), recite_compiler::authoring::Interrupted> {
        self.0.cancel.checkpoint()
    }
}
pub struct Frozen {
    identity: Identity,
    path: PathBuf,
    document: Arc<PoDocument>,
    drafts: Vec<(PoEntryId, Draft)>,
    dirty: bool,
    needs_save: bool,
}
#[derive(Default)]
pub enum Slot {
    #[default]
    Closed,
    Open(Box<Owner>),
    Working {
        ticket: Ticket,
        frozen: Option<Frozen>,
    },
}
pub struct Checkout {
    pub(super) ticket: Ticket,
    pub owner: Option<Box<Owner>>,
    pub(crate) retired: Option<Frozen>,
}
impl Slot {
    pub fn owner(&self) -> Option<&Owner> {
        match self {
            Self::Open(owner) => Some(owner),
            _ => None,
        }
    }
    pub fn owner_mut(&mut self) -> Option<&mut Owner> {
        match self {
            Self::Open(owner) if matches!(owner.access, Access::Editable) => Some(owner),
            _ => None,
        }
    }
    pub fn working(&self) -> bool {
        matches!(self, Self::Working { .. })
    }
    pub fn dirty(&self) -> bool {
        match self {
            Self::Open(owner) => owner.catalogue.dirty(),
            Self::Working {
                frozen: Some(frozen),
                ..
            } => frozen.dirty,
            _ => false,
        }
    }
    pub fn needs_save(&self) -> bool {
        match self {
            Self::Open(owner) => {
                owner.catalogue.dirty() || owner.catalogue.recovery_error().is_some()
            }
            Self::Working {
                frozen: Some(frozen),
                ..
            } => frozen.needs_save,
            _ => false,
        }
    }
    pub fn read_only(&self) -> bool {
        self.working()
            || self
                .owner()
                .is_some_and(|owner| matches!(owner.access, Access::ReopenRequired))
    }
    pub fn identity(&self) -> Option<&Identity> {
        match self {
            Self::Open(owner) => Some(&owner.identity),
            Self::Working {
                frozen: Some(frozen),
                ..
            } => Some(&frozen.identity),
            _ => None,
        }
    }
    pub fn document(&self) -> Option<&PoDocument> {
        match self {
            Self::Open(owner) => Some(&owner.catalogue.document),
            Self::Working {
                frozen: Some(frozen),
                ..
            } => Some(&frozen.document),
            _ => None,
        }
    }
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Open(owner) => Some(&owner.catalogue.path),
            Self::Working {
                frozen: Some(frozen),
                ..
            } => Some(&frozen.path),
            _ => None,
        }
    }
    pub fn draft(&self, id: PoEntryId) -> Option<Draft> {
        match self {
            Self::Open(owner) => owner.catalogue.draft(id),
            Self::Working {
                frozen: Some(frozen),
                ..
            } => frozen
                .drafts
                .iter()
                .find(|(entry, _)| *entry == id)
                .map(|(_, draft)| draft.clone()),
            _ => None,
        }
    }
    pub fn cancel(&self) {
        if let Self::Working { ticket, .. } = self {
            ticket.cancel();
        }
    }
    pub fn checkout(
        &mut self,
        selected: impl IntoIterator<Item = PoEntryId>,
    ) -> Result<Checkout, String> {
        if self.working() {
            return Err("A catalogue action is still finishing.".into());
        }
        if let Some(owner) = self.owner() {
            owner.catalogue.require_idle_save()?;
        }
        let frozen = self.owner().map(|owner| Frozen {
            identity: owner.identity.clone(),
            path: owner.catalogue.path.clone(),
            document: owner.catalogue.document.clone(),
            drafts: selected
                .into_iter()
                .filter_map(|id| owner.catalogue.draft(id).map(|draft| (id, draft)))
                .collect(),
            dirty: owner.catalogue.dirty(),
            needs_save: owner.catalogue.dirty() || owner.catalogue.recovery_error().is_some(),
        });
        let ticket = Ticket(Arc::new(Claim {
            cancel: CancellationToken::new(),
        }));
        let previous = std::mem::replace(
            self,
            Self::Working {
                ticket: ticket.clone(),
                frozen,
            },
        );
        let owner = match previous {
            Self::Open(owner) => Some(owner),
            Self::Closed => None,
            previous @ Self::Working { .. } => {
                *self = previous;
                return Err("Catalogue ownership changed.".into());
            }
        };
        Ok(Checkout {
            ticket,
            owner,
            retired: None,
        })
    }
    pub fn restore(&mut self, checkout: &mut Checkout) -> Result<(), String> {
        let Self::Working { ticket, .. } = self else {
            return Err("No catalogue operation is awaiting its owner.".into());
        };
        if !Arc::ptr_eq(&ticket.0, &checkout.ticket.0) {
            return Err("A different catalogue operation owns this slot.".into());
        }
        let next = checkout.owner.take().map_or(Self::Closed, Self::Open);
        if let Self::Working { frozen, .. } = std::mem::replace(self, next) {
            checkout.retired = frozen;
        }
        Ok(())
    }
}
