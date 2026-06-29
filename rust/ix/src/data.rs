//! Allocation-free instruction data helpers.

use crate::{
    EmberInstruction, FlightInstruction, HawkeyeInstruction, PhoenixInstruction, PhoenixIxError,
};

pub const DISCRIMINATOR_LEN: usize = 8;

pub trait InstructionDiscriminator {
    fn discriminator(self) -> [u8; DISCRIMINATOR_LEN];
}

impl InstructionDiscriminator for PhoenixInstruction {
    #[inline(always)]
    fn discriminator(self) -> [u8; DISCRIMINATOR_LEN] {
        self.discriminant()
    }
}

impl InstructionDiscriminator for EmberInstruction {
    #[inline(always)]
    fn discriminator(self) -> [u8; DISCRIMINATOR_LEN] {
        self.discriminant()
    }
}

impl InstructionDiscriminator for FlightInstruction {
    #[inline(always)]
    fn discriminator(self) -> [u8; DISCRIMINATOR_LEN] {
        self.discriminant()
    }
}

impl InstructionDiscriminator for HawkeyeInstruction {
    #[inline(always)]
    fn discriminator(self) -> [u8; DISCRIMINATOR_LEN] {
        self.discriminant()
    }
}

pub fn write_discriminator<I: InstructionDiscriminator>(
    instruction: I,
    out: &mut [u8],
) -> Result<usize, PhoenixIxError> {
    write_instruction_data(instruction, &[], out)
}

pub fn write_instruction_data<I: InstructionDiscriminator>(
    instruction: I,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, PhoenixIxError> {
    let len = DISCRIMINATOR_LEN
        .checked_add(payload.len())
        .ok_or(PhoenixIxError::InstructionDataTooLarge)?;
    if out.len() < len {
        return Err(PhoenixIxError::InstructionDataBufferTooSmall {
            expected: len,
            actual: out.len(),
        });
    }

    out[..DISCRIMINATOR_LEN].copy_from_slice(&instruction.discriminator());
    out[DISCRIMINATOR_LEN..len].copy_from_slice(payload);
    Ok(len)
}
