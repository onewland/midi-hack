# midi_hack: Naming is hard!

Don't do it.

## High-level goal

A mix of guided and free-form piano practice tools for a screen-free piano assistant.

## Milestones roughly in order

### Support for "scripts" of practice cues
What is the interaction model? 

Load from the CLI with a flag or in free mode. The most basic flow should be something like

1. Request user play X scale
1. Try playing X scale
1. Branch on correctness
    1. If correct, announce it and move onto next flow
    1. If incorrect, attempt to explain error and re-issue the request.

Maybe defined with rust macros? e.g.

```
practice_program! "major-scales-circle-of-fourths" {
    sequence keys, ["C", "F", "Bb", ...]
    // presumably, foreach_verifying only moves iteration forward if
    // there is a successful verification
    foreach_verifying key in keys { 
        prompt "play a major scale in the key of ${key}"
        listen max_notes=20, max_listen_seconds=60
        verify_major_scale key
    }
    inform "you are done"
}

practice_program! "harmonic-minor-scales-random-order" {
    sequence keys, randomize(["C", "F", "Bb", ...])
    foreach_verifying key in keys { 
        prompt if_first="play a major scale in the key of ${key}"
        listen max_notes=20, max_listen_seconds=60
        verify_harmonic_minor_scale key
    }
    inform "you are done"
}
```

`cargo run midi_hack --practice-program=major-scales-circle-of-fourths`

Should practice programs operate on MIDI signals or higher-level "music" signals?
For finding errors, it seems like they might need MIDI signals.

### TTS integration

The speech kind of sucks. Need a better solution than `say`.

#### How does sustain pedal fit here?
`key down + pedal down` is treated as `key held down` until pedal lifts

### Control with weird keys

Save or split MIDI runs, quit the application, or go to the next practice program
based on pressing key combinations that don't work musically (maybe the lowest A
repeated 4 times or something).

TODO: figure out what should actually be controlled with this

### MIDI "diff"
Detect what the problem is if a user plays a run or chord wrong e.g. "you accidentally
went up a half-step on F".

### Free-play mode: chord vs "run" grouping

### Recognizing all chord inversions efficiently

### Hand segmentation
Assign key messages to hands based on a standard person's max octave reach.

### More support for chords + scales

### Ear training improvements

- more options for multiple-note chords
- key recognition for simultaneous press or in either direction

### Tempo detection
Based on a run of notes, approximately what BPM is the player playing at? Where
are they deviating from the beat?
