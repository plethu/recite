@tool
class_name ReciteSchemaDeclarations
extends Resource

## Stable, caller-owned identity. Keep this value when the Resource moves.
@export var producer_id: String = ""
@export var types: Dictionary = {}
@export var registries: Dictionary = {}
@export var speakers: Dictionary = {}
@export var conditions: Dictionary = {}
@export var availability_reasons: Dictionary = {}
@export var effects: Dictionary = {}
@export var metadata_domains: Dictionary = {}
@export var metadata: Dictionary = {}
@export var projections: Dictionary = {}
