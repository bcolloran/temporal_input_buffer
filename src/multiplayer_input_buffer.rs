use std::collections::HashMap;

use super::{
    input_buffer::{InputStatus, PlayerInputBuffer},
    peerwise_finalized_input::PeerwiseFinalizedInput,
    util_types::{PlayerInput, PlayerInputBinary, PlayerInputSlice, PlayerNum},
};

#[derive(Debug)]
pub struct MultiplayerInputBuffers {
    max_inputs_to_predict: u32,
    num_players: u8,
    pub buffers: Vec<PlayerInputBuffer>,
}

impl Default for MultiplayerInputBuffers {
    fn default() -> Self {
        Self::new(4, 8)
    }
}

impl MultiplayerInputBuffers {
    pub fn new(num_players: u8, max_inputs_to_predict: u32) -> Self {
        Self {
            max_inputs_to_predict,
            num_players,
            buffers: (0..num_players)
                .map(|_| PlayerInputBuffer::default())
                .collect(),
        }
    }

    pub fn final_inputs_by_tick(&self) -> Vec<(u32, Vec<(u32, PlayerInput)>)> {
        let mut final_inputs = vec![];
        for tick in 0..self.get_num_finalized_inputs_across_peers() {
            let mut inputs = vec![];
            for id in self.get_peer_player_nums().iter() {
                let input = self.get_input_or_prediction(*id, tick);
                inputs.push((Into::<u32>::into(*id), input.to_input()));
                inputs.sort_by_key(|(i, _)| *i);
            }
            final_inputs.push((tick, inputs));
        }
        final_inputs
    }

    pub fn get_peer_player_nums(&self) -> Vec<PlayerNum> {
        (0..self.num_players).map(PlayerNum).collect()
    }

    pub fn get_inputs_map_for_tick(&self, tick: u32) -> HashMap<u8, PlayerInput> {
        self.buffers
            .iter()
            .enumerate()
            .map(|(player_num, buf)| {
                let input = buf.get_input_or_prediction(tick, self.max_inputs_to_predict);
                (player_num as u8, input.to_input())
            })
            .collect()
    }

    fn buffer_by_player_num(&self, player_num: PlayerNum) -> &PlayerInputBuffer {
        self.buffers
            .get::<usize>(player_num.into())
            .unwrap_or_else(|| panic!("player_num out of bounds: {:?}", player_num))
    }

    fn buffer_mut_by_player_num(&mut self, player_num: PlayerNum) -> &mut PlayerInputBuffer {
        self.buffers
            .get_mut::<usize>(player_num.into())
            .unwrap_or_else(|| panic!("player_num out of bounds: {:?}", player_num))
    }

    pub fn append_input(&mut self, player_num: PlayerNum, input: PlayerInputBinary) {
        self.buffer_mut_by_player_num(player_num)
            .append_input(input);
    }

    pub fn append_input_finalized(&mut self, player_num: PlayerNum, input: PlayerInputBinary) {
        self.buffer_mut_by_player_num(player_num)
            .host_append_finalized(input);
    }

    pub fn get_slice_to_end_for_peer(&self, player_num: PlayerNum, start: u32) -> PlayerInputSlice {
        self.buffer_by_player_num(player_num).slice_from(start)
    }

    pub fn get_input_or_prediction(&self, player_num: PlayerNum, tick: u32) -> PlayerInputBinary {
        self.buffer_by_player_num(player_num)
            .get_input_or_prediction(tick, self.max_inputs_to_predict)
    }

    /// ges the number of input for this peer, whether finalized or not
    pub fn get_num_inputs(&self, player_num: PlayerNum) -> u32 {
        self.buffer_by_player_num(player_num).num_inputs_collected()
    }

    /// gets the number of finalized inputs for this per
    pub fn get_num_finalized_inputs(&self, player_num: PlayerNum) -> u32 {
        self.buffer_by_player_num(player_num).finalized_inputs
    }

    pub fn get_num_finalized_inputs_per_peer(&self) -> HashMap<PlayerNum, u32> {
        self.buffers
            .iter()
            .enumerate()
            .map(|(player_num, buf)| (player_num.try_into().unwrap(), buf.finalized_inputs))
            .collect()
    }

    pub fn receive_peer_input_slice(&mut self, slice: PlayerInputSlice, player_num: PlayerNum) {
        self.buffer_mut_by_player_num(player_num)
            .receive_peer_input_slice(slice);
    }

    /// The host uses this method to directly append finalized default inputs such that the player has the desired number of final inputs in their buffer.
    ///
    /// Note that this is INCLUSIVE of the tick.
    pub fn append_final_default_inputs_to_target(
        &mut self,
        player_num: PlayerNum,
        target_num: u32,
    ) {
        self.buffer_mut_by_player_num(player_num)
            .host_append_final_default_inputs_to_target(target_num);
    }

    pub fn receive_finalized_input_slice_for_player(
        &mut self,
        slice: PlayerInputSlice,
        player_num: PlayerNum,
    ) {
        self.buffer_mut_by_player_num(player_num)
            .receive_finalized_input_slice(slice);
    }

    /// This method builds the PeerwiseFinalizedInput mapping
    /// based on this buffer's state.
    pub fn get_peerwise_finalized_inputs(&self) -> PeerwiseFinalizedInput {
        let ack = PeerwiseFinalizedInput::new(
            self.buffers
                .iter()
                .enumerate()
                .map(|(player_num, buf)| (player_num.try_into().unwrap(), buf.finalized_inputs))
                .collect(),
        );
        ack
    }

    pub fn buffer_len_per_player(&self) -> HashMap<PlayerNum, u32> {
        self.buffers
            .iter()
            .enumerate()
            .map(|(player_num, buf)| (player_num.try_into().unwrap(), buf.inputs.len() as u32))
            .collect()
    }

    /// Return the number of inputs that have been finalized for all players, i.e., the `min_i {f_i}` where `f_i` is the number of finalized inputs for player i.
    ///
    ///  Reminder of indexing conventions: If we have seen 0 finalized inputs for all players, we can only snapshot the initial state at tick 0; if we have seen 1 finalized input for all players, we can snapshot up to tick 1; generally, if we have seen T finalized inputs for all players, we can snapshot up to tick T; and this means that the index in the input buffer is T-1.
    pub fn get_num_finalized_inputs_across_peers(&self) -> u32 {
        self.buffers
            .iter()
            .map(|buf| buf.finalized_inputs)
            .min()
            .unwrap_or(0)
    }

    /// For each player, returns the inputs for the given tick and whether the inputs have been finalized.
    pub fn get_inputs_and_finalization_status(&self, tick: u32) -> Vec<(u8, PlayerInput, bool)> {
        let mut inputs: Vec<_> = self
            .buffers
            .iter()
            .enumerate()
            .map(|(player_num, buf)| {
                let input = buf.get_input_or_prediction(tick, self.max_inputs_to_predict);
                (player_num as u8, input.to_input(), buf.is_finalized(tick))
            })
            .collect();
        inputs.sort_by_key(|(i, _, _)| *i);
        inputs
    }

    /// For each player, returns the InputStatus for the given input_num
    pub fn get_inputs_status(&self, input_num: u32) -> Vec<(PlayerNum, InputStatus)> {
        self.buffers
            .iter()
            .enumerate()
            .map(|(player_num, buf)| ((player_num as u8).into(), buf.get_input_status(input_num)))
            .collect()
    }

    pub fn serialize_player_buffer(
        &self,
        player_num: PlayerNum,
        reset_finalization: bool,
    ) -> Vec<u8> {
        let buf = self.buffer_by_player_num(player_num);
        if reset_finalization {
            let mut buf = buf.clone();
            buf.finalized_inputs = 0;
            return bincode::serialize(&buf).unwrap();
        }
        bincode::serialize(buf).unwrap()
    }
    pub fn deserialize_player_buffer(&mut self, player_num: PlayerNum, data: &[u8]) {
        let buf: PlayerInputBuffer = bincode::deserialize(data).unwrap();
        let num: usize = player_num.into();
        self.buffers[num] = buf;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_and_get_input() {
        let mut buffers = MultiplayerInputBuffers::default();
        buffers.append_input(1.into(), PlayerInputBinary::new_test_simple(42));

        let slice = buffers.get_slice_to_end_for_peer(1.into(), 0);
        assert_eq!(slice.inputs, vec![PlayerInputBinary::new_test_simple(42)]);
        assert_eq!(slice.start, 0);
    }

    #[test]
    fn test_finalized_ticks() {
        let mut buffers = MultiplayerInputBuffers::default();
        buffers.append_input_finalized(1.into(), PlayerInputBinary::new_test_simple(42));

        assert_eq!(buffers.get_num_finalized_inputs(1.into()), 1);
        assert_eq!(buffers.get_num_finalized_inputs(2.into()), 0);

        let finalized_ticks = buffers.get_num_finalized_inputs_per_peer();
        assert_eq!(finalized_ticks.get(&1.into()), Some(&1u32));
    }

    #[test]
    fn test_get_num_finalized_inputs_across_peers() {
        let mut buffers = MultiplayerInputBuffers::new(2, 8);

        assert_eq!(buffers.get_num_finalized_inputs_across_peers(), 0);

        buffers.append_input_finalized(0.into(), PlayerInputBinary::new_test_simple(0));

        // peer 0 has 1 finalized input, across all peers we still have 0
        assert_eq!(buffers.get_num_finalized_inputs_across_peers(), 0);

        for t in 1..5 {
            buffers.append_input_finalized(0.into(), PlayerInputBinary::new_test_simple(t));
        }

        // peer 0 has 5 finalized input, across all peers we still have 0
        assert_eq!(buffers.get_num_finalized_inputs_across_peers(), 0);

        buffers.append_input_finalized(1.into(), PlayerInputBinary::new_test_simple(0));
        assert_eq!(buffers.get_num_finalized_inputs_across_peers(), 1);

        for t in 0..10 {
            buffers.append_input_finalized(1.into(), PlayerInputBinary::new_test_simple(t));
        }

        assert_eq!(buffers.get_num_finalized_inputs_across_peers(), 5);
    }

    #[test]
    fn test_buffer_len_per_player() {
        let mut buffers = MultiplayerInputBuffers::default();
        buffers.append_input(1.into(), PlayerInputBinary::new_test_simple(42));
        buffers.append_input(1.into(), PlayerInputBinary::new_test_simple(43));

        buffers.append_input(2.into(), PlayerInputBinary::new_test_simple(44));
        buffers.append_input(2.into(), PlayerInputBinary::new_test_simple(44));
        buffers.append_input(2.into(), PlayerInputBinary::new_test_simple(44));
        buffers.append_input(2.into(), PlayerInputBinary::new_test_simple(44));

        let lengths = buffers.buffer_len_per_player();
        assert_eq!(lengths.get(&1.into()), Some(&2));
        assert_eq!(lengths.get(&2.into()), Some(&4));
    }

    #[test]
    fn test_receive_peer_input_slice() {
        let mut buffers = MultiplayerInputBuffers::default();
        let slice = PlayerInputSlice {
            start: 0,
            inputs: vec![
                PlayerInputBinary::new_test_simple(1),
                PlayerInputBinary::new_test_simple(2),
            ],
        };

        buffers.receive_peer_input_slice(slice.clone(), 1.into());

        let retrieved = buffers.get_slice_to_end_for_peer(1.into(), 0);
        assert_eq!(retrieved.inputs, slice.inputs);
        assert_eq!(retrieved.start, 0);
    }

    #[test]
    fn test_host_append_default_inputs() {
        let mut buffers = MultiplayerInputBuffers::default();
        buffers.append_final_default_inputs_to_target(1.into(), 4);

        assert_eq!(buffers.get_num_finalized_inputs(1.into()), 5);

        let slice = buffers.get_slice_to_end_for_peer(1.into(), 0);
        assert_eq!(slice.inputs.len(), 5);
    }

    #[test]
    fn test_receive_finalized_input_slice() {
        let mut buffers = MultiplayerInputBuffers::default();
        let slice = PlayerInputSlice {
            start: 0,
            inputs: vec![
                PlayerInputBinary::new_test_simple(1),
                PlayerInputBinary::new_test_simple(2),
            ],
        };

        buffers.receive_finalized_input_slice_for_player(slice, 1.into());
        assert_eq!(buffers.get_num_finalized_inputs(1.into()), 2);
    }

    #[test]
    fn test_get_peerwise_finalized_inputs() {
        let mut buffers = MultiplayerInputBuffers::default();
        buffers.append_input_finalized(1.into(), PlayerInputBinary::new_test_simple(1));
        buffers.append_input_finalized(2.into(), PlayerInputBinary::new_test_simple(1));
        buffers.append_input_finalized(2.into(), PlayerInputBinary::new_test_simple(2));

        let pfi_map = buffers.get_peerwise_finalized_inputs().inner();
        assert_eq!(pfi_map.get(&1.into()), Some(&1));
        assert_eq!(pfi_map.get(&2.into()), Some(&2));
    }
}
