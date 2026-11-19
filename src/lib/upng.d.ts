declare module "upng-js" {
  const png: {
    encode(
      frames: ArrayBufferLike[],
      width: number,
      height: number,
      colors: number,
    ): ArrayBuffer;
  };
  export default png;
}
