#!/usr/bin/env python3
"""
Extract Protobuf definitions from Cursor's workbench.desktop.main.js

Uses hybrid regex + heuristics approach to parse minified JS and reconstruct
protobuf message schemas.
"""

import re
import json
import argparse
from pathlib import Path
from dataclasses import dataclass, field, asdict
from typing import Optional

# Protobuf scalar type codes
SCALAR_TYPES = {
    1: "double",
    2: "float",
    3: "int64",
    4: "uint64",
    5: "int32",
    6: "fixed64",
    7: "fixed32",
    8: "bool",
    9: "string",
    10: "group",
    11: "message",
    12: "bytes",
    13: "uint32",
    14: "enum",
    15: "sfixed32",
    16: "sfixed64",
    17: "sint32",
    18: "sint64",
}


@dataclass
class ProtoField:
    number: int
    name: str
    kind: str  # scalar, enum, message
    type_code: Optional[int] = None
    type_name: Optional[str] = None
    repeated: bool = False
    optional: bool = False
    oneof: Optional[str] = None


@dataclass
class ProtoMessage:
    name: str
    full_name: str
    fields: list[ProtoField] = field(default_factory=list)
    nested_messages: list[str] = field(default_factory=list)
    nested_enums: list[str] = field(default_factory=list)


@dataclass
class ProtoEnum:
    name: str
    full_name: str
    values: dict[str, int] = field(default_factory=dict)


class ProtobufExtractor:
    def __init__(self, js_content: str):
        self.content = js_content
        self.messages: dict[str, ProtoMessage] = {}
        self.enums: dict[str, ProtoEnum] = {}
        self.var_to_type: dict[str, str] = {}  # Maps JS variables to type names

    def extract_all(self):
        """Main extraction pipeline."""
        self._extract_type_registrations()
        self._extract_message_definitions()
        self._extract_enum_definitions()
        self._resolve_type_references()
        return self

    def _extract_type_registrations(self):
        """Find all typeName="aiserver.v1.XXX" registrations."""
        # Pattern: typeName:"aiserver.v1.MessageName" or typeName="aiserver.v1.MessageName"
        pattern = r'typeName[=:]["\'](aiserver\.v1\.[^"\']+)["\']'

        for match in re.finditer(pattern, self.content):
            full_name = match.group(1)
            short_name = full_name.split('.')[-1]

            if full_name not in self.messages:
                self.messages[full_name] = ProtoMessage(
                    name=short_name,
                    full_name=full_name
                )

            # Try to find the variable name associated with this type
            # Look backwards for class/const definition
            start = max(0, match.start() - 500)
            context = self.content[start:match.start()]

            # Look for patterns like: const Xyz = ... or class Xyz
            var_match = re.search(r'(?:const|let|var|class)\s+(\w+)\s*=?\s*[^;]*$', context)
            if var_match:
                self.var_to_type[var_match.group(1)] = full_name

    def _extract_message_definitions(self):
        """Extract field definitions for each message."""
        # Primary pattern: this.typeName="xxx"}static{this.fields=v.util.newFieldList(()=>[...])
        # This is the protobuf-es runtime pattern used by Cursor

        pattern = (
            r'this\.typeName="(aiserver\.v1\.[^"]+)"[^}]*\}'
            r'static\{this\.fields=\w+\.util\.newFieldList\(\(\)=>\[([^\]]+)\]\)'
        )

        for match in re.finditer(pattern, self.content):
            full_name = match.group(1)
            fields_str = match.group(2)

            short_name = full_name.split('.')[-1]
            fields = self._extract_fields_from_context(fields_str)

            if full_name not in self.messages:
                self.messages[full_name] = ProtoMessage(
                    name=short_name,
                    full_name=full_name,
                    fields=fields
                )
            elif len(fields) > len(self.messages[full_name].fields):
                self.messages[full_name].fields = fields

        # Fallback: Look for standalone makeMessageType patterns
        self._extract_standalone_field_definitions()

    def _extract_fields_from_context(self, context: str) -> list[ProtoField]:
        """Extract field definitions from a code context."""
        fields_by_key: dict[tuple[int, str], ProtoField] = {}

        # Pattern for individual field objects
        # {no:1,name:"text",kind:"scalar",T:9}
        # {no:2,name:"type",kind:"enum",T:e.getEnumType(Va)}
        # {no:3,name:"chunks",kind:"message",T:Oce,repeated:!0}

        field_pattern = r'\{no:(\d+),name:"([^"]+)",kind:"(\w+)"([^}]*)\}'

        for match in re.finditer(field_pattern, context):
            field_no = int(match.group(1))
            field_name = match.group(2)
            field_kind = match.group(3)
            extra = match.group(4)

            proto_field = ProtoField(
                number=field_no,
                name=field_name,
                kind=field_kind
            )

            # Extract type code for scalars
            type_match = re.search(r',T:(\d+)', extra)
            if type_match:
                proto_field.type_code = int(type_match.group(1))

            # Extract type reference for messages/enums
            type_ref_match = re.search(r',T:(\w+)', extra)
            if type_ref_match and not type_match:
                proto_field.type_name = type_ref_match.group(1)

            # Check for repeated
            if 'repeated:!0' in extra or 'repeated:true' in extra.lower():
                proto_field.repeated = True

            # Check for optional
            if 'opt:!0' in extra or 'optional:!0' in extra:
                proto_field.optional = True

            # Check for oneof
            oneof_match = re.search(r'oneof:"([^"]+)"', extra)
            if oneof_match:
                proto_field.oneof = oneof_match.group(1)

            # Deduplicate by (field_number, field_name)
            key = (field_no, field_name)
            if key not in fields_by_key:
                fields_by_key[key] = proto_field

        return sorted(fields_by_key.values(), key=lambda f: f.number)

    def _extract_standalone_field_definitions(self):
        """Find field definition arrays that may not be near type names."""
        # Look for runtime message creation patterns
        # e.g., proto3.makeMessageType("aiserver.v1.Xxx", [...fields...])

        pattern = r'makeMessageType\s*\(\s*["\']([^"\']+)["\']\s*,\s*\[([^\]]+)\]'

        for match in re.finditer(pattern, self.content):
            full_name = match.group(1)
            fields_str = '[' + match.group(2) + ']'

            if full_name.startswith('aiserver.v1.'):
                fields = self._extract_fields_from_context(fields_str)

                if full_name not in self.messages:
                    short_name = full_name.split('.')[-1]
                    self.messages[full_name] = ProtoMessage(
                        name=short_name,
                        full_name=full_name,
                        fields=fields
                    )
                elif fields and len(fields) > len(self.messages[full_name].fields):
                    self.messages[full_name].fields = fields

    def _extract_enum_definitions(self):
        """Extract enum type definitions."""
        # Pattern: makeEnum("aiserver.v1.EnumName", [{no:0,name:"XXX"},{no:1,name:"YYY"}])
        pattern = r'makeEnum\s*\(\s*["\']([^"\']+)["\']\s*,\s*\[([^\]]+)\]'

        for match in re.finditer(pattern, self.content):
            full_name = match.group(1)
            values_str = match.group(2)

            if full_name.startswith('aiserver.v1.'):
                short_name = full_name.split('.')[-1]
                enum = ProtoEnum(name=short_name, full_name=full_name)

                # Extract enum values
                value_pattern = r'\{no:(\d+),name:"([^"]+)"'
                for val_match in re.finditer(value_pattern, values_str):
                    enum.values[val_match.group(2)] = int(val_match.group(1))

                self.enums[full_name] = enum

    def _resolve_type_references(self):
        """Try to resolve variable references to actual type names."""
        for message in self.messages.values():
            for field in message.fields:
                if field.type_name and field.type_name in self.var_to_type:
                    field.type_name = self.var_to_type[field.type_name]

    def to_json(self) -> str:
        """Export as JSON."""
        data = {
            "messages": {k: asdict(v) for k, v in sorted(self.messages.items())},
            "enums": {k: asdict(v) for k, v in sorted(self.enums.items())},
        }
        return json.dumps(data, indent=2)

    def to_proto(self) -> str:
        """Export as .proto file format."""
        lines = [
            'syntax = "proto3";',
            '',
            'package aiserver.v1;',
            '',
        ]

        # Add enums first
        for enum in sorted(self.enums.values(), key=lambda e: e.name):
            lines.append(f'enum {enum.name} {{')
            for name, number in sorted(enum.values.items(), key=lambda x: x[1]):
                lines.append(f'  {name} = {number};')
            lines.append('}')
            lines.append('')

        # Add messages
        for message in sorted(self.messages.values(), key=lambda m: m.name):
            lines.append(f'message {message.name} {{')

            # Group oneof fields
            oneofs: dict[str, list[ProtoField]] = {}
            regular_fields: list[ProtoField] = []

            for field in message.fields:
                if field.oneof:
                    if field.oneof not in oneofs:
                        oneofs[field.oneof] = []
                    oneofs[field.oneof].append(field)
                else:
                    regular_fields.append(field)

            # Write regular fields
            for field in regular_fields:
                field_line = self._format_field(field)
                lines.append(f'  {field_line}')

            # Write oneof groups
            for oneof_name, oneof_fields in oneofs.items():
                lines.append(f'  oneof {oneof_name} {{')
                for field in oneof_fields:
                    field_line = self._format_field(field, in_oneof=True)
                    lines.append(f'    {field_line}')
                lines.append('  }')

            lines.append('}')
            lines.append('')

        return '\n'.join(lines)

    def _format_field(self, field: ProtoField, in_oneof: bool = False) -> str:
        """Format a single field as proto syntax."""
        # Determine type string
        if field.kind == 'scalar' and field.type_code:
            type_str = SCALAR_TYPES.get(field.type_code, f'unknown_{field.type_code}')
        elif field.kind == 'message':
            if field.type_name:
                # Extract just the message name if it's a full path
                type_str = field.type_name.split('.')[-1] if '.' in str(field.type_name) else str(field.type_name)
            else:
                type_str = 'bytes'  # fallback
        elif field.kind == 'enum':
            if field.type_name:
                type_str = field.type_name.split('.')[-1] if '.' in str(field.type_name) else str(field.type_name)
            else:
                type_str = 'int32'  # fallback for unknown enum
        else:
            type_str = 'bytes'  # ultimate fallback

        # Build field declaration
        prefix = 'repeated ' if field.repeated else ''
        if field.optional and not in_oneof:
            prefix = 'optional ' + prefix

        return f'{prefix}{type_str} {field.name} = {field.number};'

    def print_summary(self):
        """Print extraction summary."""
        print(f"\n{'='*60}")
        print(f"Extraction Summary")
        print(f"{'='*60}")
        print(f"Messages found: {len(self.messages)}")
        print(f"Enums found: {len(self.enums)}")
        print(f"Variable mappings: {len(self.var_to_type)}")

        print(f"\n{'='*60}")
        print("Messages:")
        print(f"{'='*60}")
        for name, msg in sorted(self.messages.items()):
            field_count = len(msg.fields)
            print(f"  {msg.name}: {field_count} fields")
            for f in msg.fields[:5]:  # Show first 5 fields
                type_info = SCALAR_TYPES.get(f.type_code, f.type_name or f.kind)
                repeated = " (repeated)" if f.repeated else ""
                print(f"    - {f.number}: {f.name} [{type_info}]{repeated}")
            if field_count > 5:
                print(f"    ... and {field_count - 5} more fields")

        if self.enums:
            print(f"\n{'='*60}")
            print("Enums:")
            print(f"{'='*60}")
            for name, enum in sorted(self.enums.items()):
                print(f"  {enum.name}: {len(enum.values)} values")


def main():
    parser = argparse.ArgumentParser(
        description='Extract Protobuf definitions from Cursor JS bundle'
    )
    parser.add_argument(
        'input',
        nargs='?',
        default='/Applications/Cursor.app/Contents/Resources/app/out/vs/workbench/workbench.desktop.main.js',
        help='Path to workbench.desktop.main.js'
    )
    parser.add_argument(
        '-o', '--output',
        help='Output file path (default: stdout)'
    )
    parser.add_argument(
        '-f', '--format',
        choices=['json', 'proto', 'summary'],
        default='summary',
        help='Output format'
    )

    args = parser.parse_args()

    # Read input file
    input_path = Path(args.input)
    if not input_path.exists():
        print(f"Error: File not found: {input_path}")
        return 1

    print(f"Reading {input_path}...")
    content = input_path.read_text(encoding='utf-8', errors='ignore')
    print(f"File size: {len(content):,} bytes")

    # Extract protobuf definitions
    extractor = ProtobufExtractor(content)
    extractor.extract_all()

    # Output results
    if args.format == 'summary':
        extractor.print_summary()
        output = None
    elif args.format == 'json':
        output = extractor.to_json()
    elif args.format == 'proto':
        output = extractor.to_proto()

    if output:
        if args.output:
            Path(args.output).write_text(output)
            print(f"Output written to {args.output}")
        else:
            print(output)

    return 0


if __name__ == '__main__':
    exit(main())
