{
  mkShellNoCC,
  procxy,
}:
mkShellNoCC {
  inputsFrom = [ procxy ];
}
