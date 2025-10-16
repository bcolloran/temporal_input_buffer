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
    pub fn new(map: HashMap<PlayerNum, u32>) -> Self {
        Self(map)
    }
    pub fn inner(&self) -> HashMap<PlayerNum, u32> {
        self.0.clone()
    }

    pub fn get(&self, player_num: PlayerNum) -> u32 {
        self.0.get(&player_num).copied().unwrap_or(0)
    }

    /// Update the ack with the ticks from another ack
    /// if the other ack has a newer tick for the same player_num.
    pub fn update(&mut self, other: PeerwiseFinalizedInputsSeen) {
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

    /// Returns a new FinalizedTicksAck where each entry is the oldest of the two
    /// ticks for the same player_num
    pub fn pairwise_oldest(
        &self,
        other: &PeerwiseFinalizedInputsSeen,
    ) -> PeerwiseFinalizedInputsSeen {
        let mut ack = self.clone();

        for (player_num, tick) in other.0.iter() {
            if let Some(existing_tick) = ack.0.get(player_num) {
                if tick < existing_tick {
                    ack.0.insert(*player_num, *tick);
                }
            } else {
                ack.0.insert(*player_num, *tick);
            }
        }
        ack
    }

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

        let ack = PeerwiseFinalizedInputsSeen::new(map);
        assert_eq!(ack.get(1.into()), 10);
        assert_eq!(ack.get(2.into()), 20);
        assert_eq!(ack.get(3.into()), 0);
    }

    #[test]
    fn test_update() {
        let mut ack1 =
            PeerwiseFinalizedInputsSeen::new(HashMap::from([(1.into(), 10), (2.into(), 20)]));
        let ack2 = PeerwiseFinalizedInputsSeen::new(HashMap::from([
            (1.into(), 15),
            (2.into(), 15),
            (3.into(), 25),
        ]));

        ack1.update(ack2);
        assert_eq!(ack1.get(1.into()), 15);
        assert_eq!(ack1.get(2.into()), 20);
        assert_eq!(ack1.get(3.into()), 25);
    }

    #[test]
    fn test_pairwise_oldest() {
        let ack1 =
            PeerwiseFinalizedInputsSeen::new(HashMap::from([(1.into(), 10), (2.into(), 20)]));
        let ack2 = PeerwiseFinalizedInputsSeen::new(HashMap::from([
            (1.into(), 15),
            (2.into(), 15),
            (3.into(), 25),
        ]));

        let result = ack1.pairwise_oldest(&ack2);
        assert_eq!(result.get(1.into()), 10);
        assert_eq!(result.get(2.into()), 15);
        assert_eq!(result.get(3.into()), 25);
    }
}
