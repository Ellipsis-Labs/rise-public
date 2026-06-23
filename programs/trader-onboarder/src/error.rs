use pinocchio::program_error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraderOnboarderError {
    InvalidInstruction       = 6000,
    InvalidAccountList       = 6001,
    InvalidAccountKey        = 6002,
    InvalidAccountOwner      = 6003,
    MissingRequiredSignature = 6004,
    AccountBorrowFailed      = 6005,
    AccountDeserializeFailed = 6006,
    InvalidAccountData       = 6007,
    PermissionUnavailable    = 6008,
    PhoenixIxBuildFailed     = 6009,
    PhoenixCpiMismatch       = 6010,
}

impl TraderOnboarderError {
    pub const fn code(self) -> u32 {
        self as u32
    }
}

impl From<TraderOnboarderError> for ProgramError {
    fn from(error: TraderOnboarderError) -> Self {
        ProgramError::Custom(error.code())
    }
}
