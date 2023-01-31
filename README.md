# slice

a cli utility that grabs an arbitrary size slice out of a file

still a work in progress!

## usage

    slice [OPTIONS] <input>

#### arguments

* **\<input\>** : path of the file to read

#### options

* -o, --output **\<output\>** : file to output (default: stdout)
* -n, --bytes **\<bytes\>** : number of bytes to read (default: all)
* -s, --skip **\<skip\>** : number of bytes to skip (default: 0)
* -e, --end **\<end\>** : byte to stop reading on
* -h, --help : print help
* -V, --version : print version
