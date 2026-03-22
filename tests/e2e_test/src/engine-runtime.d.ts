// LLM3dEngine runtime type declarations
// Provides global types available in the QuickJS environment

declare function print(...args: any[]): void;

declare namespace console {
    function log(...args: any[]): void;
}
