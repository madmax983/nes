#!/bin/bash
find crates -type f -name "*.rs" -exec grep -H "TODO" {} +
