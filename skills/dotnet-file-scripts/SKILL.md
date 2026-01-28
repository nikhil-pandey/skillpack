---
name: dotnet-file-scripts
description: Write or update single-file C# apps run with `dotnet run <file>.cs` (.NET 10+ file-based apps), including `#:package`, `#:sdk`, `#:property`, shebang scripts, or converting to a project with `dotnet project convert`.
---

# Dotnet file-based scripts

Use file-based apps. Keep code single-file unless user asks for a project.

## Workflow

1. Confirm goal, target runtime (.NET 10 preview if unspecified), and filename.
2. Provide a complete `.cs` file with top-level statements.
3. Add directives at file top, before `using` lines.
4. Include run commands and any required `chmod` for shebang scripts.
5. If user needs multi-file or tooling, propose project conversion.

## Directives

Use when needed, one per line at top:

```csharp
#:package Humanizer@2.14.1
#:sdk Microsoft.NET.Sdk.Web
#:property LangVersion preview
```

## Shebang scripts

For executable scripts on Unix:

```csharp
#!/usr/bin/dotnet run
Console.WriteLine("Hello from C#");
```

Add commands:

```bash
chmod +x app.cs
./app.cs
```

## Project conversion

When code grows beyond a single file or needs full tooling:

```bash
dotnet project convert app.cs
```

Explain that this creates a folder, `.csproj`, and moves code to `Program.cs`, translating directives to MSBuild.
