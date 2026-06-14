# eslint-plugin-rustify

ESLint rules for the TypeScript subset supported by Rustify.

## Flat config

```js
import rustify from "eslint-plugin-rustify";

export default [rustify.configs["flat/recommended"]];
```

## Legacy config

```json
{
  "extends": ["plugin:rustify/recommended"]
}
```

Rules:

- `rustify/no-any`
- `rustify/no-unknown`
- `rustify/no-eval`
- `rustify/no-dynamic-object`
- `rustify/no-unsupported-union`
- `rustify/no-unsupported-syntax`
- `rustify/explicit-return-type`
- `rustify/explicit-param-types`

`rustify/no-any` safely autofixes `any` on immutable primitive literal constants
to the inferred `string`, `number`, or `boolean` type. Other `any` usages remain
diagnostics because Rustify cannot infer a safe replacement.

The recommended config also rejects native-incompatible classes, function
expressions, arrow functions, exception handling, dynamic `this`, prototype
access, and monkey patching.
