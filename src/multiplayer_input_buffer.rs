use std::collections::HashMap;

use crate::input_trait::SimInput;

use super::{
    input_buffer::{InputStatus, PlayerInputBuffer},
    peerwise_finalized_input::PeerwiseFinalizedInputsSeen,
    util_types::{PlayerInputSlice, PlayerNum},
};

#[derive(Debug)]
pub struct MultiplayerInputBuffers<T>
where
    T: SimInput,
{
    max_inputs_to_predict: u32,
    num_players: u8,
    pub buffers: Vec<PlayerInputBuffer<T>>,
}

impl<T: SimInput> Default for MultiplayerInputBuffers<T> {
    fn default() -> Self {
        Self::new(4, 8)
    }
}

impl<T: SimInput> MultiplayerInputBuffers<T> {
    pub fn new(num_players: u8, max_inputs_to_predict: u32) -> Self {
        Self {
            max_inputs_to_predict,
            num_players,
            buffers: (0..num_players)
                .map(|_| PlayerInputBuffer::default())
                .collect(),
        }
    }

    pub fn final_inputs_by_tick(&self) -> Vec<(u32, Vec<(u32, T)>)> {
        let mut final_inputs = vec![];
        for tick in 0..self.get_num_finalized_inputs_across_peers() {
            let mut inputs = vec![];
            for id in self.get_peer_player_nums().iter() {
                let input = self.get_input_or_prediction(*id, tick);
                inputs.push((Into::<u32>::into(*id), input));
                inputs.sort_by_key(|(i, _)| *i);
            }
            final_inputs.push((tick, inputs));
        }
        final_inputs
    }

    pub fn get_peer_player_nums(&self) -> Vec<PlayerNum> {
        (0..self.num_players).map(PlayerNum).collect()
    }

    pub fn get_inputs_map_for_tick(&self, tick: u32) -> HashMap<u8, T> {
        self.buffers
            .iter()
            .enumerate()
            .map(|(player_num, buf)| {
                let input = buf.get_input_or_prediction(tick, self.max_inputs_to_predict);
                (player_num as u8, input)
            })
            .collect()
    }

    fn buffer_by_player_num(&self, player_num: PlayerNum) -> &PlayerInputBuffer<T> {
        self.buffers
            .get::<usize>(player_num.into())
            .unwrap_or_else(|| panic!("player_num out of bounds: {:?}", player_num))
    }

    fn buffer_mut_by_player_num(&mut self, player_num: PlayerNum) -> &mut PlayerInputBuffer<T> {
        self.buffers
            .get_mut::<usize>(player_num.into())
            .unwrap_or_else(|| panic!("player_num out of bounds: {:?}", player_num))
    }

    pub fn append_input(&mut self, player_num: PlayerNum, input: T) {
        self.buffer_mut_by_player_num(player_num)
            .append_input(input.to_bytes());
    }

    pub fn append_input_finalized(&mut self, player_num: PlayerNum, input: T) {
        self.buffer_mut_by_player_num(player_num)
            .host_append_finalized(input.to_bytes());
    }

    pub fn get_slice_to_end_for_peer(
        &self,
        player_num: PlayerNum,
        start: u32,
    ) -> PlayerInputSlice<T> {
        self.buffer_by_player_num(player_num).slice_from(start)
    }

    pub fn get_input_or_prediction(&self, player_num: PlayerNum, tick: u32) -> T {
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

    pub fn receive_peer_input_slice(&mut self, slice: PlayerInputSlice<T>, player_num: PlayerNum) {
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
        slice: PlayerInputSlice<T>,
        player_num: PlayerNum,
    ) {
        self.buffer_mut_by_player_num(player_num)
            .receive_finalized_input_slice(slice);
    }

    /// This method builds the PeerwiseFinalizedInput mapping
    /// based on this buffer's state.
    pub fn get_peerwise_finalized_inputs(&self) -> PeerwiseFinalizedInputsSeen {
        let ack = PeerwiseFinalizedInputsSeen::new(
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
    pub fn get_inputs_and_finalization_status(&self, tick: u32) -> Vec<(u8, T, bool)> {
        let mut inputs: Vec<_> = self
            .buffers
            .iter()
            .enumerate()
            .map(|(player_num, buf)| {
                let input = buf.get_input_or_prediction(tick, self.max_inputs_to_predict);
                (player_num as u8, input, buf.is_finalized(tick))
            })
            .collect();
        inputs.sort_by_key(|(i, _, _)| *i);
        inputs
    }

    /// For each player, returns the InputStatus for the given input_num
    pub fn get_input_statuses(&self, input_num: u32) -> Vec<(PlayerNum, InputStatus)> {
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
        let buf: PlayerInputBuffer<T> = bincode::deserialize(data).unwrap();
        let num: usize = player_num.into();
        self.buffers[num] = buf;
    }
}
