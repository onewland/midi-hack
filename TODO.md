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

### Efficient time-sensitive note store
Types of queries to support:

1. Between t=~1.8 and t=~2.2, what are all notes currently being held?
1. Between t=~5 and t=~7, what key up messages were observed in what order?
1. Given two notes in a sequence that I know were played, when did the run start and finish?
1. "Is this key not being pressed in this time range?" should be extremely fast as it is maybe the most common condition being checked
 
We don't want to require queries to be overly precise because people don't
play with extreme precision. Something played at t=1.004s should be be considered
"simultaneous" to something at t=1s for most queries.

#### How does sustain pedal fit here?
`key down + pedal down` is treated as `key held down` until pedal lifts

#### How is the data stored?

This is an example of a Bb major chord being played:

        A1 Bb1 B1 C2 C#2 D2 Eb2 E2 F2 ...
      |----------------------------------
b = 0 |    1 
b = 1 |    1 
b = 2 |    1             1
b = 3 |    1             1
b = 4 |    1             1          1
b = 5 |    1             1          1
b = 6 |                             1 

Time is bucketd to `b` to the left by something like 100ns-1ms so that we
don't mistake simultaneous playing for non-simultaneous.

Entry into the data store should store in at least the prior and upcoming time bucket
to allow for "fuzzy matching".

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

### Use MIDI out for ear training
Play a note sequence or chord using the device of the musician, and have them
try to mimic. Maybe provide hints in between.

### Tempo detection
Based on a run of notes, approximately what BPM is the player playing at? Where
are they deviating from the beat?

## Milestones Finished