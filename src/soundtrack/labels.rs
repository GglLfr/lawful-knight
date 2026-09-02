use crate::soundtrack_label;

soundtrack_label! {
    #[derive(Debug, Clone, Copy)]
    pub enum BehindTheMirror {
        #[path = "behind_the_mirror_arp.ogg"]
        Arp,
        #[path = "behind_the_mirror_bass_arp.ogg"]
        BassArp,
        #[path = "behind_the_mirror_bass.ogg"]
        Bass,
        #[path = "behind_the_mirror_combat_bass.ogg"]
        CombatBass,
        #[path = "behind_the_mirror_combat_bells.ogg"]
        CombatBells,
        #[path = "behind_the_mirror_combat_guitar.ogg"]
        CombatGuitar,
        #[path = "behind_the_mirror_combat_percussion.ogg"]
        CombatPercussion,
        #[path = "behind_the_mirror_harp.ogg"]
        Harp,
        #[path = "behind_the_mirror_string_section.ogg"]
        StringSection,
        #[path = "behind_the_mirror_string_solo.ogg"]
        StringSolo,
    }
}
