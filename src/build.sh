#!/bin/bash

code="$PWD"
opts=-g
cd idk > /dev/null
g++ $opts $code/what -o  jgfld
cd $code > /dev/null
