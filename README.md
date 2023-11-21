# midi_hack

A silly little program for piano practice using MIDI.

It uses the say command on MacOS to give verbal instructions,
then verifies whether you followed those instructions
correctly.

## How to run

If you only have one MIDI device, you can leave off
the `--midi-device-port=n` flag.

### Ear training 
This requires your MIDI device to support both in
and out. It plays two notes on your device and then
you should play them back.

To re-hear the notes, play the lowest "A" key on the
piano twice.

```
cargo run --bin=midi_hack --package=midi_hack -- \
 --midi-device-port=0 ear-training
```


### Circle of Fourths, major scales
```
cargo run --bin=midi_hack --package=midi_hack circle-of-fourths --midi-device-port=0 
```