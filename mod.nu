#!/usr/bin/env nu

export def helper [] {
  let cond = {|x| true }
  [{a: 1}] | where $cond
}

export def main [] {
  'ok'
}
