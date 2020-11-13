#!make
include .env
export $(shell sed 's/=.*//' .env)

.PHONY: run

run:
	cargo run