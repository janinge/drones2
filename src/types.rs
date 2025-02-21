use std::num::{NonZeroI16, NonZeroU8};

pub type NodeId = u8;
pub type Time = i16;
pub type Capacity = i32;
pub type Cost = i32;
pub type CargoSize = u16;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CallId(NonZeroI16);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VehicleId(NonZeroU8);

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

impl TryFrom<usize> for CallId {
    type Error = &'static str;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let i: i16 = value.try_into().map_err(|_| "Value too large for CallId")?;
        if i == 0 {
            Err("CallId can not be zero")
        } else {
            NonZeroI16::new(i)
                .map(CallId)
                .ok_or("Failed to create CallId")
        }
    }
}

impl VehicleId {
    /// Creates a new VehicleId from a 1-indexed u8 value.
    /// Returns None if input is 0.
    #[inline(always)]
    pub fn new(value: u8) -> Option<Self> {
        NonZeroU8::new(value).map(VehicleId)
    }

    /// Creates a new VehicleId from a 0-indexed usize.
    /// Returns None if the conversion would result in overflow.
    #[inline(always)]
    pub fn from_index(idx: usize) -> Option<Self> {
        let value = idx.checked_add(1)?;
        if value > u8::MAX as usize {
            None
        } else {
            NonZeroU8::new(value as u8).map(VehicleId)
        }
    }

    /// Returns the raw vehicle ID value (1-indexed).
    #[inline(always)]
    pub fn get(self) -> u8 {
        self.0.get()
    }

    /// Returns the 0-indexed version of this vehicle ID for array indexing.
    #[inline(always)]
    pub fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

impl TryFrom<usize> for VehicleId {
    type Error = &'static str;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value == 0 {
            return Err("VehicleId cannot be zero");
        }
        if value > u8::MAX as usize {
            return Err("Value too large for VehicleId");
        }
        NonZeroU8::new(value as u8)
            .map(VehicleId)
            .ok_or("Failed to create VehicleId")
    }
}

impl PartialEq<u8> for VehicleId {
    fn eq(&self, other: &u8) -> bool {
        self.0.get() == *other
    }
}
