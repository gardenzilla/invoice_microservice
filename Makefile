#!make
include .env
export $(shell sed 's/=.*//' .env)

.PHONY: build

build:
	cargo run