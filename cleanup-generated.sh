#!/bin/bash
git status --porcelain | awk '{print $2}' | xargs -r rm -rf
