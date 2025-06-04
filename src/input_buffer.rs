use super::util_types::{PlayerInputBinary, PlayerInputSlice};

use serde::{Deserialize, Serialize};

/// The status of the inputs for a given tick.
pub enum InputStatus {
    /// Recieved from a peer and finalized by the host.
    Finalized,
    /// Recieved from a peer, but not yet finalized.
    NonFinal,
    /// Not yet recieved from a peer.
    NotRecieved,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerInputBuffer {
    /// The number of inputs that have been finalized.
    pub finalized_inputs: u32,
    pub inputs: Vec<PlayerInputBinary>,
}

impl PlayerInputBuffer {
    pub fn is_finalized(&self, tick: u32) -> bool {
        tick < self.finalized_inputs
    }

    pub fn num_inputs_collected(&self) -> u32 {
        self.inputs.len() as u32
    }

    pub fn append_input(&mut self, input: PlayerInputBinary) {
        self.inputs.push(input);
    }

    /// The host uses this method to directly append a finalized input
    /// to it's own buffer.
    pub fn host_append_finalized(&mut self, input: PlayerInputBinary) {
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
    fn set_next_final(&mut self, index: u32, input: PlayerInputBinary) {
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
            self.set_next_final(t, PlayerInputBinary::default());
        }
    }

    pub fn get_input_or_prediction(
        &self,
        tick: u32,
        max_ticks_to_predict_locf: u32,
    ) -> PlayerInputBinary {
        if tick < self.inputs.len() as u32 {
            // if the tick is within the buffer, return the input.
            // Do this no matter whether the input has been finalized or not;
            // even if it's a local input, it's better than predicting.
            self.inputs[tick as usize]
        } else if self.inputs.len() > 0
            && (tick < self.inputs.len() as u32 + max_ticks_to_predict_locf)
        {
            // if there is no input for this tick, in the buffer,
            // but we've collected at least one input, and
            // we are within the prediction window, return the last
            // observed input (even if it's not finalized, it's the best we have)
            self.inputs[self.inputs.len() - 1]
        } else {
            // if we are outside the prediction window, return default
            PlayerInputBinary::default()
        }
    }

    pub fn get_input_status(&self, input_num: u32) -> InputStatus {
        if input_num <= self.finalized_inputs {
            InputStatus::Finalized
        } else if input_num < self.inputs.len() as u32 {
            InputStatus::NonFinal
        } else {
            InputStatus::NotRecieved
        }
    }

    /// gets slice from tick start to end. EXCLUSIVE
    pub fn slice(&self, start: u32, end: u32) -> PlayerInputSlice {
        PlayerInputSlice {
            inputs: self.inputs[start as usize..end as usize].to_vec(),
            start,
        }
    }

    pub fn slice_from(&self, start: u32) -> PlayerInputSlice {
        PlayerInputSlice {
            inputs: self.inputs[start as usize..self.inputs.len()].to_vec(),
            start,
        }
    }

    /// This method is used to update the buffer when a peer sends
    /// a slice of inputs that have not yet been finalized.
    pub fn receive_peer_input_slice(&mut self, slice: PlayerInputSlice) {
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
    pub fn receive_finalized_input_slice(&mut self, slice: PlayerInputSlice) {
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

//
//
//
//
//
//
// tests
//
//

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_buffer_basics() {
        let mut buffer = PlayerInputBuffer::default();
        assert_eq!(buffer.num_inputs_collected(), 0);
        assert_eq!(buffer.finalized_inputs, 0);

        let input = PlayerInputBinary::default();
        buffer.append_input(input);
        assert_eq!(buffer.num_inputs_collected(), 1);
        assert_eq!(buffer.finalized_inputs, 0);
    }

    #[test]
    fn test_host_append_finalized() {
        let mut buffer = PlayerInputBuffer::default();
        let input = PlayerInputBinary::default();

        buffer.host_append_finalized(input);
        assert_eq!(buffer.finalized_inputs, 1);
        assert_eq!(buffer.num_inputs_collected(), 1);
    }

    #[test]
    fn test_get_input_or_prediction() {
        let mut buffer = PlayerInputBuffer::default();
        // default if nothing yet in buffer,
        // for any combination of tick and max_ticks_to_predict_locf
        assert_eq!(
            buffer.get_input_or_prediction(0, 0),
            PlayerInputBinary::default()
        );
        assert_eq!(
            buffer.get_input_or_prediction(0, 10),
            PlayerInputBinary::default()
        );
        assert_eq!(
            buffer.get_input_or_prediction(10, 10),
            PlayerInputBinary::default()
        );
        assert_eq!(
            buffer.get_input_or_prediction(0, 0),
            PlayerInputBinary::default()
        );

        buffer.append_input(PlayerInputBinary::new_test_simple(0));
        buffer.append_input(PlayerInputBinary::new_test_simple(1));
        buffer.append_input(PlayerInputBinary::new_test_simple(2));
        buffer.append_input(PlayerInputBinary::new_test_simple(3));
        buffer.append_input(PlayerInputBinary::new_test_simple(4));

        assert_eq!(
            buffer.get_input_or_prediction(0, 5),
            PlayerInputBinary::new_test_simple(0)
        );
        assert_eq!(
            buffer.get_input_or_prediction(1, 5),
            PlayerInputBinary::new_test_simple(1)
        );
        assert_eq!(
            buffer.get_input_or_prediction(5, 5),
            PlayerInputBinary::new_test_simple(4)
        );
        assert_eq!(
            buffer.get_input_or_prediction(9, 5),
            PlayerInputBinary::new_test_simple(4)
        );
        assert_eq!(
            buffer.get_input_or_prediction(10, 5),
            PlayerInputBinary::default()
        );
    }

    #[test]
    fn test_receive_finalized_input_slice() {
        let mut buffer = PlayerInputBuffer::default();
        let slice = PlayerInputSlice::new_test(0, 5);

        buffer.receive_finalized_input_slice(slice);
        assert_eq!(buffer.finalized_inputs, 5);
        assert_eq!(buffer.num_inputs_collected(), 5);

        // Test slice with gap (should be ignored)
        let slice_with_gap = PlayerInputSlice::new_test(6, 5);
        buffer.receive_finalized_input_slice(slice_with_gap);
        assert_eq!(buffer.finalized_inputs, 5);
    }

    #[test]
    fn test_receive_peer_input_slice() {
        let mut buffer = PlayerInputBuffer::default();

        // zero finalized inputs so far
        assert_eq!(buffer.finalized_inputs, 0);

        buffer.receive_finalized_input_slice(PlayerInputSlice::new_test(0, 2));

        // now we have 2 finalized inputs
        assert_eq!(buffer.finalized_inputs, 2);

        // rx a slice of inputs that have not been finalized
        let slice = PlayerInputSlice::new_test(0, 5);

        // the buffer should now have 5 inputs, but still only 2 finalized
        buffer.receive_peer_input_slice(slice);
        assert_eq!(buffer.num_inputs_collected(), 5);
        assert_eq!(buffer.finalized_inputs, 2);

        // rx 4 more finalized inputs
        buffer.receive_finalized_input_slice(PlayerInputSlice::new_test(2, 4));
        // now we have 6 inputs, and all of them are finalized
        assert_eq!(buffer.num_inputs_collected(), 6);
        assert_eq!(buffer.finalized_inputs, 6);
    }

    #[test]
    fn test_rx_out_of_order_final_slices() {
        let mut buffer = PlayerInputBuffer::default();

        // add 5 default inputs
        buffer.receive_finalized_input_slice(PlayerInputSlice {
            start: 0,
            inputs: vec![
                PlayerInputBinary::default(),
                PlayerInputBinary::default(),
                PlayerInputBinary::default(),
                PlayerInputBinary::default(),
                PlayerInputBinary::default(),
            ],
        });

        // now rx a finalized slice that starts at 0
        buffer.receive_finalized_input_slice(PlayerInputSlice {
            start: 0,
            inputs: vec![
                PlayerInputBinary::new_test_simple(10),
                PlayerInputBinary::new_test_simple(20),
                PlayerInputBinary::new_test_simple(30),
                PlayerInputBinary::new_test_simple(40),
                PlayerInputBinary::new_test_simple(50),
            ],
        });

        // make sure the buffer still has the original inputs
        assert_eq!(buffer.num_inputs_collected(), 5);
        assert_eq!(buffer.finalized_inputs, 5);
        for i in 0..5 {
            assert_eq!(buffer.inputs[i], PlayerInputBinary::default());
        }
    }

    #[test]
    fn test_host_finalize_default_thru_tick() {
        let mut buffer = PlayerInputBuffer::default();
        buffer.host_append_final_default_inputs_to_target(4);

        assert_eq!(buffer.num_inputs_collected(), 5);
        assert_eq!(buffer.finalized_inputs, 5);
        for i in 0..5 {
            assert_eq!(buffer.inputs[i], PlayerInputBinary::default());
        }
    }

    #[test]
    fn test_host_finalize_default_thru_tick_wont_overwrite() {
        let mut buffer = PlayerInputBuffer::default();
        buffer.receive_finalized_input_slice(PlayerInputSlice::new_test(0, 5));
        for i in 0..5 {
            assert_eq!(
                buffer.inputs[i],
                PlayerInputBinary::new_test_simple(i as u8)
            );
        }

        buffer.host_append_final_default_inputs_to_target(4);

        // the buffer should still have the original inputs
        assert_eq!(buffer.num_inputs_collected(), 5);
        assert_eq!(buffer.finalized_inputs, 5);
        for i in 0..5 {
            assert_eq!(
                buffer.inputs[i],
                PlayerInputBinary::new_test_simple(i as u8)
            );
        }
    }
}
