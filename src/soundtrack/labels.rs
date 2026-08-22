use crate::soundtrack_label;

soundtrack_label! {
    #[derive(Copy)]
    pub enum BehindTheMirror {
        #[path = "behind_the_mirror_arp.ogg"]
        Arp,
        #[path = "behind_the_mirror_bass_arp.ogg"]
        BassArp,
        #[path = "behind_the_mirror_bass.ogg"]
        Bass,
        #[path = "behind_the_mirror_harp.ogg"]
        Harp,
        #[path = "behind_the_mirror_pad.ogg"]
        Pad,
        #[path = "behind_the_mirror_viola.ogg"]
        Viola,
        #[path = "behind_the_mirror_violin_i.ogg"]
        Violin,
        #[path = "behind_the_mirror_violins_i.ogg"]
        Violins,
        #[path = "behind_the_mirror_whistle.ogg"]
        Whistle,
    }
}
