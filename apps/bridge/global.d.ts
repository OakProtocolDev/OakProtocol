declare namespace JSX {
  interface IntrinsicElements {
    "stargate-widget": React.DetailedHTMLProps<
      React.HTMLAttributes<HTMLElement> & {
        theme?: string;
        "data-destination-chain-id"?: number;
      },
      HTMLElement
    >;
  }
}
