// Copyright (C) 2015-2026 The Neo Project.
//
// NeoVmExecutionContextAccessors.cs file belongs to the neo project and is free
// software distributed under the MIT software license, see the
// accompanying file LICENSE in the main directory of the
// repository or http://www.opensource.org/licenses/mit-license.php
// for more details.

using Neo.VM;
using Neo.VM.Types;
using System;
using System.Reflection;

namespace Neo.SmartContract.RiscV
{
    internal static class NeoVmExecutionContextAccessors
    {
        private static readonly FieldInfo? StrictModeField =
            typeof(Script).GetField("_strictMode", BindingFlags.Instance | BindingFlags.NonPublic);

        private static readonly FieldInfo? CurrentContextField =
            typeof(ExecutionEngine).GetField("<CurrentContext>k__BackingField", BindingFlags.Instance | BindingFlags.NonPublic);

        private static readonly FieldInfo? InstructionPointerField =
            typeof(ExecutionContext).GetField("instructionPointer", BindingFlags.Instance | BindingFlags.NonPublic)
            ?? typeof(ExecutionContext).GetField("_instructionPointer", BindingFlags.Instance | BindingFlags.NonPublic)
            ?? typeof(ExecutionContext).GetField("<InstructionPointer>k__BackingField", BindingFlags.Instance | BindingFlags.NonPublic);

        private static readonly MethodInfo? SetLocalVariablesMethod =
            typeof(ExecutionContext)
                .GetProperty("LocalVariables", BindingFlags.Instance | BindingFlags.Public)
                ?.GetSetMethod(nonPublic: true);

        internal static bool StrictModeAccessorAvailableForTesting => StrictModeField is not null;

        internal static bool CurrentContextAccessorAvailableForTesting => CurrentContextField is not null;

        internal static bool FaultInstructionPointerAccessorAvailableForTesting => InstructionPointerField is not null;

        internal static bool FaultLocalVariablesAccessorAvailableForTesting => SetLocalVariablesMethod is not null;

        internal static bool IsStrictMode(Script script)
        {
            var field = StrictModeField
                ?? throw new InvalidOperationException("Unable to locate Neo.VM.Script strict mode field.");

            return (bool)field.GetValue(script)!;
        }

        internal static void SetCurrentContext(ExecutionEngine engine, ExecutionContext? context)
        {
            var field = CurrentContextField
                ?? throw new InvalidOperationException("Unable to locate Neo.VM.ExecutionEngine current context field.");

            field.SetValue(engine, context);
        }

        internal static bool TrySetInstructionPointer(ExecutionContext? context, int instructionPointer, Action<string>? trace = null)
        {
            if (context is null)
                return false;

            if (InstructionPointerField is null)
            {
                trace?.Invoke("failed to propagate fault IP: Neo.VM.ExecutionContext instruction pointer field was not found");
                return false;
            }

            try
            {
                InstructionPointerField.SetValue(context, instructionPointer);
                return true;
            }
            catch (Exception ex)
            {
                trace?.Invoke($"failed to propagate fault IP {instructionPointer}: {ex.Message}");
                return false;
            }
        }

        internal static bool TrySetLocalVariables(ExecutionContext? context, Slot localVariables, Action<string>? trace = null)
        {
            if (context is null)
                return false;

            if (SetLocalVariablesMethod is null)
            {
                trace?.Invoke("failed to propagate fault locals: Neo.VM.ExecutionContext local variables setter was not found");
                return false;
            }

            try
            {
                SetLocalVariablesMethod.Invoke(context, [localVariables]);
                return true;
            }
            catch (Exception ex)
            {
                trace?.Invoke($"failed to propagate fault locals: {ex.Message}");
                return false;
            }
        }
    }
}
