# slice

a cli utility that grabs an arbitrary size slice out of a file

still a work in progress!

## usage

    slice [OPTIONS] <input>

#### arguments

- \<input\> : path of the file to read

#### options

- **-o, --output** \<output\> : \file to output to. default: stdout
- **-n, --bytes** \<bytes\> : number of bytes to read. default: all
- **-s, --start** \<start\> : byte to start reading at (inclusive). default: 0
- **-e, --end** \<end\> : byte to stop reading at (exclusive). default: last byte
- **-h, --help** : print help
- **-V, --version** : print version
