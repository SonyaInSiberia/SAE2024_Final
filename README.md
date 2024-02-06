# SAE2024_Final
Project Description to be submitted here

### Plans for implementation - needed components
#### Sampler Engine 🎛️
- Process parameter and MIDI Events and render the audio
- Allocate voices attached to a file or manage the number of active voices (voice stealing)
- Load or configure sound banks, such as SFZ, sf2, aiff...
#### ADSR Module 📉 (Attack Decay Sustain Release)
- Shapes the volume of the signal over time through an envelope generator or another trigger. See code reference of `adsr.c` from [7].
#### Sampler Voice 🎹
- An object which holds a reference to a sample mapped to a keyboard and defines basic properties of said sample (i.e. root note, length, loop point, start point, etc.)
#### Audio Buffer 🗄️
- A low level data structure which can stream a sample from disc and load it into memory

### Plans for implementation - potential need for 3rd party libs
- [1] “dasp.” GitHub, Nov. 11, 2023. Accessed: Feb. 06, 2024. [Online]. Available: https://github.com/RustAudio/dasp/tree/master

### Algorithmic references - which reference do you base your algorithmic implementations on?
- [2] W. C. Pirkle, Designing software synthesizer plug-ins in C++ with audio DSP, 2nd edition. New York: Routledge, 2021.
- [3] S. Dunne, “DunneCore Sampler.” AudioKit, GitHub, May 27, 2021. Accessed: Feb. 05, 2024. [iOS/macOS]. Available: https://github.com/AudioKit/DunneAudioKit/commits/main/Sources/CDunneAudioKit/DunneCore/Sampler/README.md
- [4] N. Tanaka, “RustySynth.” GitHub, Aug. 01, 2023. Accessed: Feb. 06, 2024. [Online]. Available: https://github.com/sinshu/rustysynth?tab=readme-ov-file
- [5] M. Puckette, Theory and Techniques of Electronic Music. in http://msp.ucsd.edu/techniques/v0.01/book-html/book.html. University of California, San Diego, 2003.
- [7] P. Batchelor, “Soundpipe.” Nov. 07, 2023. Accessed: Feb. 06, 2024. [Online]. Available: https://paulbatchelor.github.io/proj/soundpipe.html

### Other references
- [6] J. Ćavar and L. Dolecki, “Exploring AU Sampler - Apple’s Mysterious Sampler Audio Unit.” Accessed: Feb. 06, 2024. [Online]. Available: https://infinum.com/blog/getting-started-with-au-sampler/
