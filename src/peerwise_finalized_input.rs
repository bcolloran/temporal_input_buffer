use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

use super::util_types::PlayerNum;

/// For each peer, the number of inputs that have
/// been finalized by the host *and that the peer who
/// sent this ack has seen* ie., that they have in their
/// own local MultiplayerInputBuffer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeerwiseFinalizedInputsSeen(HashMap<PlayerNum, u32>);

impl Display for PeerwiseFinalizedInputsSeen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FinalizedInputsSeen(")?;
        let mut sorted = self.0.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|&(p, _)| p);
        for (player_num, tick) in &sorted {
            write!(f, "{}={} ", player_num, tick)?;
        }
        write!(f, ")")
    }
}

impl PeerwiseFinalizedInputsSeen {
    pub fn new(num_players: u8) -> Self {
        Self(HashMap::from_iter(
            (0..num_players).map(|i| (PlayerNum(i), 0)),
        ))
    }

    pub fn new_from_observed(num_players: u8, observed: &[u32]) -> Self {
        assert!(observed.len() as u8 == num_players);
        let mut map = HashMap::new();
        for (i, &tick) in observed.iter().enumerate() {
            map.insert(PlayerNum(i as u8), tick);
        }
        Self(map)
    }

    #[cfg(test)]
    pub fn new_test(map: HashMap<PlayerNum, u32>) -> Self {
        Self(map)
    }
    pub fn inner(&self) -> HashMap<PlayerNum, u32> {
        self.0.clone()
    }

    /// Get the number of finalized inputs seen for a given player_num.
    pub fn get(&self, player_num: PlayerNum) -> u32 {
        self.0.get(&player_num).copied().unwrap_or(0)
    }

    /// Update the ack with the ticks from another ack
    /// if the other ack has a newer tick for the same player_num.
    ///
    /// FIXME: was encountering a bug where the input buffer was being re-initialized on guests, which caused the clients PeerwiseFinalizedInputsSeen to be reset to zeroes. This meant that when the host rxed the observation from the guest, because we only merge NEWER ticks, the host would ignore the guest's observation entirely, since all the ticks were zeroes. And thus, there would be a gap between what the host thought the guest had finalized and what the guest thought it had finalized. And this gap would never be filled, b/c the host would never send inputs prior to what it thought the guest had finalized.
    /// To work around this, for now we just always update to the other ack's ticks regardless of whether they are newer or older.
    pub fn merge_needs_to_be_fixed(&mut self, other: PeerwiseFinalizedInputsSeen) {
        *self = other;
    }

    /// Update the ack with the ticks from another ack
    /// if the other ack has a newer tick for the same player_num.
    ///
    /// FIXME: use this version!!
    pub fn merge(&mut self, other: PeerwiseFinalizedInputsSeen) {
        for (player_num, tick) in other.0.iter() {
            if let Some(existing_tick) = self.0.get(player_num) {
                if tick > existing_tick {
                    self.0.insert(*player_num, *tick);
                }
            } else {
                self.0.insert(*player_num, *tick);
            }
        }
    }

    // /// Returns a new PeerwiseFinalizedInputsSeen where each entry is the oldest of the two
    // /// ticks for the same player_num
    // pub fn pairwise_oldest(
    //     &self,
    //     other: &PeerwiseFinalizedInputsSeen,
    // ) -> PeerwiseFinalizedInputsSeen {
    //     let mut ack = self.clone();

    //     for (player_num, tick) in other.0.iter() {
    //         if let Some(existing_tick) = ack.0.get(player_num) {
    //             if tick < existing_tick {
    //                 ack.0.insert(*player_num, *tick);
    //             }
    //         } else {
    //             ack.0.insert(*player_num, *tick);
    //         }
    //     }
    //     ack
    // }

    pub fn earliest_input_finalized_by_all(&self) -> u32 {
        self.0.values().copied().min().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{peerwise_finalized_input::PeerwiseFinalizedInputsSeen, util_types::PlayerNum};

    #[test]
    fn test_basic_operations() {
        let mut map = HashMap::new();
        map.insert(PlayerNum(1), 10);
        map.insert(PlayerNum(2), 20);

        let ack = PeerwiseFinalizedInputsSeen::new_test(map);
        assert_eq!(ack.get(1.into()), 10);
        assert_eq!(ack.get(2.into()), 20);
        assert_eq!(ack.get(3.into()), 0);
    }

    #[test]
    fn test_update() {
        let mut ack1 =
            PeerwiseFinalizedInputsSeen::new_test(HashMap::from([(1.into(), 10), (2.into(), 20)]));
        let ack2 = PeerwiseFinalizedInputsSeen::new_test(HashMap::from([
            (1.into(), 15),
            (2.into(), 15),
            (3.into(), 25),
        ]));

        ack1.merge(ack2);
        assert_eq!(ack1.get(1.into()), 15);
        assert_eq!(ack1.get(2.into()), 20);
        assert_eq!(ack1.get(3.into()), 25);
    }
}
