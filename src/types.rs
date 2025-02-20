use std::num::{NonZeroI16, NonZeroU8};

pub type VehicleId = NonZeroU8;
pub type NodeId = u8;
pub type Time = i16;
pub type Capacity = u32;
pub type Cost = i32;
pub type CargoSize = u16;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CallId(NonZeroI16);

impl CallId {
    /// Creates a new CallId from a raw i16.
    /// Returns None if input is 0 (because NonZeroI16 can't hold zero).
    pub fn new_pickup(value: i16) -> Option<Self> {
        debug_assert!(value > 0, "Pickup call ID must be positive");
        NonZeroI16::new(value).map(CallId)
    }

    pub fn new_delivery(value: i16) -> Option<Self> {
        debug_assert!(value > 0, "Delivery call ID must be positive");
        NonZeroI16::new(-value).map(CallId)
    }

    /// Returns the call ID as a positive integer.
    #[inline(always)]
    pub fn id(self) -> i16 {
        self.0.get().abs()
    }

    /// Returns the underlying i16 value.
    #[inline(always)]
    pub fn raw(self) -> i16 {
        self.0.get()
    }

    #[inline(always)]
    pub fn pickup(self) -> Self {
        CallId(NonZeroI16::new(self.raw().abs()).unwrap())
    }

    #[inline(always)]
    pub fn delivery(self) -> Self {
        CallId(NonZeroI16::new(-self.raw().abs()).unwrap())
    }

    /// Returns `true` if this call represents a pickup
    #[inline(always)]
    pub fn is_pickup(self) -> bool {
        self.raw() > 0
    }

    /// Returns `true` if this call represents a delivery
    #[inline(always)]
    pub fn is_delivery(self) -> bool {
        self.raw() < 0
    }

    /// Returns the inverse call ID (pickup → delivery and vice versa).
    #[inline(always)]
    pub fn inverse(self) -> Self {
        CallId(NonZeroI16::new(-self.raw()).unwrap())
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        self.id() as usize - 1
    }
}
