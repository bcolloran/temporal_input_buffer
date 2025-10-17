use super::{peerwise_finalized_input::PeerwiseFinalizedInputsSeen, util_types::PlayerNum};

/// Tracks the number of finalized input ticks that each GUEST has acked for each other peer, including the host. This is used to determine how many inputs the host needs to broadcast upon RXing inputs from a peer (including the host itself).
///
/// An instance of this struct is owned by the HOST. Guests do not need to track this information.
///
///
/// Keys: player_num of GUEST
/// Values: the PeerwiseFinalizedInput of for each other peer,
/// as seen by this GUEST.
pub struct FinalizedObservationsPerGuest(Vec<PeerwiseFinalizedInputsSeen>);

impl FinalizedObservationsPerGuest {
    pub fn new(num_players: u8) -> Self {
        let num_guests = num_players - 1;
        let vec = (0..num_guests)
            .map(|_guest_idx| PeerwiseFinalizedInputsSeen::new(num_players))
            .collect::<Vec<_>>();
        Self(vec)
    }

    /// For the target player_num, get the minimum number of finalized inputs observed by any guest for that player_num.
    ///
    /// Since every guest will have observed at least this many many finalized inputs for the the target player_num, if the host sends a finalized input slice to all players starting from this tick, then all guests will be able to up to the end of that slice withuout leaving gaps.
    pub(super) fn get_earliest_num_observed_final_for_peer(&self, player_num: PlayerNum) -> u32 {
        self.0.iter().map(|v| v.get(player_num)).min().unwrap_or(0)
    }

    /// Update the observation for a given guest player_num with a new PeerwiseFinalizedInputsSeen.
    ///
    /// In case observations arrive out of order, we merge the new observation with the existing one, keeping the maximum tick observed for each peer. FIXME: see comment in PeerwiseFinalizedInputsSeen::merge_needs_to_be_fixed about a bug that caused us to have to use the "needs_to_be_fixed" version of merge.
    pub fn update_guest_observation(
        &mut self,
        guest_player_num: PlayerNum,
        observation: PeerwiseFinalizedInputsSeen,
    ) {
        let guest_idx = guest_player_num
            .guest_index()
            .expect("not a guest player_num");

        self.0[guest_idx].merge_needs_to_be_fixed(observation);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{peerwise_finalized_input::PeerwiseFinalizedInputsSeen, util_types::PlayerNum};

    #[test]
    fn test_earliest_num_observed_final_for_peer() {
        let mut map = HashMap::new();
        map.insert(
            PlayerNum(1),
            PeerwiseFinalizedInputsSeen::new_test(HashMap::from_iter([
                (PlayerNum(0), 3),
                (PlayerNum(1), 5),
                (PlayerNum(2), 4),
            ])),
        );
    }
}
