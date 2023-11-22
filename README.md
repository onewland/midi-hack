# midi_hack

A silly little program for piano practice using MIDI. 

The primary goal is to not require you to look at the screen while playing.
It uses the say command on MacOS to give verbal instructions, then verifies
(verbally) whether you followed those instructions correctly.

## How to run

If you only have one MIDI device, you can leave off the `--midi-device-port=n` flag.

Press 'q' and enter in the console to quit. 'p' prints the MIDI messages
currently stored in memory. `n` clears that buffer.

### Ear training 
This requires your MIDI device to support both in and out. It plays two notes 
on your device and then you should play them back (lowest note first).

To re-hear the notes, play the lowest "A" key on the
piano twice.

```
cargo run --bin=midi_hack --package=midi_hack -- --midi-device-port=0 ear-training
```


### Circle of Fourths, major scales
Tests that you can do one octave up-and-down, no expectations on tempo, going
through the scales in fourths-intervals.

```
cargo run --bin=midi_hack --package=midi_hack -- --midi-device-port=0 circle-of-fourths 
```

### Free play
Free play mode recognizes major and harmonic minor scales. It's mostly useful
for debugging.

```
cargo run --bin=midi_hack --package=midi_hack -- --midi-device-port=0 free-play
```