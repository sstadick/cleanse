# 🔥 cleanse

A small utility to clean up delimited data to make it consumable by standard unix tools.

## Search words

Clean tsv data.
Clean csv data.

## Overview

Under the hood this uses the `csv` crate to parse data as a CSV, respecting quoting and escaping rules. For each field 
`cleanse` will then try to do the following three things:

1. Inside a field, replace any instances of the `delimiter` character with ` `.
2. Inside a field, replace any instances of the terminator `\n` character with ` `.
3. Inside a field, replace any malformed UTF8 with the utf8 replacment character.

If any changes were made to a field a log entry is made with the record number, field number and changes.


## Example

```bash
$ cat data.tsv | cleanse -o cleansed.tsv -
Aug 18 15:28:02.556  INFO cleanse: Record number 23485, field number 35: [TerminatorReplacement]
Aug 18 15:28:02.724  INFO cleanse: Record number 31036, field number 24: [DelimiterReplacement]
Aug 18 15:28:02.984  INFO cleanse: Record number 44053, field number 35: [TerminatorReplacement]
Aug 18 15:28:03.456  INFO cleanse: Record number 66273, field number 35: [TerminatorReplacement]
Aug 18 15:28:05.149  INFO cleanse: Record number 150669, field number 14: [FixedEncoding]

```


