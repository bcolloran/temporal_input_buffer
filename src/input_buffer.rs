use crate::{
    input_messages::from_bincode_bytes, input_trait::SimInput, util_types::PlayerInputSlice,
};

use serde::{Deserialize, Serialize};

/// The status of the inputs for a given tick.
pub enum InputStatus {
    /// Received from a peer and finalized by the host.
    Finalized,
    /// Received from a peer, but not yet finalized.
    NonFinal,
    /// Not yet received from a peer.
    NotReceived,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerInputBuffer<T>
where
    T: SimInput,
{
    /// The number of inputs that have been finalized.
    pub finalized_inputs: u32,
    pub inputs: Vec<T::Bytes>,
}

impl<T> PlayerInputBuffer<T>
where
    T: SimInput,
{
    pub fn from_bincode_bytes(bytes: &[u8]) -> Self {
        let decoded = from_bincode_bytes::<Self>(bytes);
        match decoded {
            Ok(buffer) => buffer,
            Err(e) => panic!("Failed to decode PlayerInputBuffer: {}", e),
        }
    }

    pub fn is_finalized(&self, tick: u32) -> bool {
        tick < self.finalized_inputs
    }

    pub fn num_inputs_collected(&self) -> u32 {
        self.inputs.len() as u32
    }

    pub fn append_input(&mut self, input: T::Bytes) {
        self.inputs.push(input);
    }

    /// The host uses this method to directly append a finalized input
    /// to it's own buffer.
    pub fn host_append_finalized(&mut self, input: T::Bytes) {
        self.set_next_final(self.finalized_inputs, input);
    }

    /// ALWAYS USE THIS TO FINALIZE INPUTS
    ///
    /// This method is used to finalize an input at a specific index,
    /// but because
    /// (a) we don't want to overwrite already finalized inputs, and
    /// (b) we don't want to leave gaps in the finalized input history,
    /// we need to check that the index being finalized is the next
    /// input in the sequence.
    ///
    /// These checks are necessary because the buffer can receive
    /// slices out of order
    fn set_next_final(&mut self, index: u32, input: T::Bytes) {
        if index != self.finalized_inputs {
            // if not finalizing the next input, do nothing--
            // would either leave a gap or overwrite a finalized input
            return;
        }

        // we can increment the number of finalized inputs
        self.finalized_inputs += 1;

        if index == self.inputs.len() as u32 {
            // if we are finalizing the next input for the buffer,
            // just append it
            self.inputs.push(input);
        } else if index < self.inputs.len() as u32 {
            self.inputs[index as usize] = input;
        } else {
            // we should never get here
            panic!("Tried to finalize an input that doesn't exist");
        }
    }

    /// The host uses this method to directly append finalized default inputs such that the player has the desired number of final inputs in their buffer.
    ///
    /// Note that this is INCLUSIVE of the target.
    pub fn host_append_final_default_inputs_to_target(&mut self, target: u32) {
        // we want an input for index `target`, so we need the
        // buffer to have len `target+1`. So stop appending at `target`
        for t in self.finalized_inputs..=target {
            self.set_next_final(t, T::default().to_bytes());
        }
    }

    pub fn get_input_or_prediction(&self, tick: u32, max_ticks_to_predict_locf: u32) -> T {
        if tick < self.inputs.len() as u32 {
            // if the tick is within the buffer, return the input.
            // Do this no matter whether the input has been finalized or not;
            // even if it's a local input, it's better than predicting.
            T::from_bytes(self.inputs[tick as usize])
        } else if self.inputs.len() > 0
            && (tick < self.inputs.len() as u32 + max_ticks_to_predict_locf)
        {
            // if there is no input for this tick, in the buffer,
            // but we've collected at least one input, and
            // we are within the prediction window, return the last
            // observed input (even if it's not finalized, it's the best we have)
            T::from_bytes(self.inputs[self.inputs.len() - 1])
        } else {
            // if we are outside the prediction window, return default
            T::default()
        }
    }

    pub fn get_input_status(&self, input_num: u32) -> InputStatus {
        if input_num < self.finalized_inputs {
            InputStatus::Finalized
        } else if input_num < self.inputs.len() as u32 {
            InputStatus::NonFinal
        } else {
            InputStatus::NotReceived
        }
    }

    /// gets slice from tick start to end. EXCLUSIVE
    pub fn slice(&self, start: u32, end: u32) -> PlayerInputSlice<T> {
        PlayerInputSlice {
            inputs: self.inputs[start as usize..end as usize].to_vec(),
            start,
        }
    }

    pub fn slice_from(&self, start: u32) -> PlayerInputSlice<T> {
        PlayerInputSlice {
            inputs: self.inputs[start as usize..self.inputs.len()].to_vec(),
            start,
        }
    }

    /// This method is used to update the buffer when a peer sends
    /// a slice of inputs that have not yet been finalized.
    pub fn receive_peer_input_slice(&mut self, slice: PlayerInputSlice<T>) {
        // just append these potentially temporary inputs after the last
        // finalized input
        let start = slice.start as usize;
        for (offset, input) in slice.inputs.iter().enumerate() {
            let t = start + offset;
            // we don't want to overwrite finalized inputs, so only update
            // the buffer with non-finalized inputs for ticks *after*
            // the last finalized input
            //
            // Note that if weve seen t+1 finalized inputs, the index of the
            // newest finalized input is t, so we can write to index t+1
            if t + 1 > self.finalized_inputs as usize {
                if t < self.inputs.len() {
                    self.inputs[t] = *input
                } else {
                    // add additional inputs
                    self.inputs.push(*input);
                }
            }
        }
    }

    /// This method is used to update the buffer when the server
    /// sends a slice of inputs that have been finalized.
    pub fn receive_finalized_input_slice(&mut self, slice: PlayerInputSlice<T>) {
        // this is a no-op if it would leave a gap in the finalized
        // input history, so the new data must overlap or start
        // with the next tick that hasn't yet been finalized.
        // If this condition is not met, we will keep requesting
        // inputs slices starting at finalized_input until one arrives.
        if slice.start > self.finalized_inputs {
            return;
        }

        let start = slice.start as usize;
        // at this point, we know the slice starts before or at the next tick
        // that hasn't been finalized, so we can append it
        for (offset, input) in slice.inputs.iter().enumerate() {
            let t = start + offset;
            self.set_next_final(t as u32, *input);
        }
    }
}
