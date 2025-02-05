"use client";

import { Card, CardBody } from "@nextui-org/react";
import { useGlobalContext } from "@/app/core/context";
import Dashboard from "./pages/dashboard/page";

export default function Home() {
  const { authenticated } = useGlobalContext();

  return <>{authenticated ? <Dashboard /> : <Welcome />}</>;
}

const Welcome = () => {
  return (
    <section className="flex flex-col items-center justify-center gap-1 md:py-1">
      <div className="inline-block max-w-lg text-center justify-center">
        <h1 className="mb-4 text-2xl font-extrabold leading-none tracking-tight text-gray-900 md:text-5xl lg:text-3xl dark:text-white">
          Welcome to Escher
        </h1>
        <Card>
          <CardBody>
            <p>Please connect to your wallet</p>
          </CardBody>
        </Card>
      </div>
    </section>
  );
};
