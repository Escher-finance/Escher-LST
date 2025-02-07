export default function TransactionsLayout({
    children,
}: {
    children: React.ReactNode;
}) {
    return (
        <section className="flex flex-col justify-center gap-2 py-4 md:py-10">
            <div className="inline-block text-center justify-center">
                {children}
            </div>
        </section>
    );
}
