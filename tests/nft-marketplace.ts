import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { NftMarketplace } from "../target/types/nft_marketplace";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { 
    createMint, 
    createAccount, 
    mintTo, 
    getAssociatedTokenAddress, 
    TOKEN_PROGRAM_ID, 
    ASSOCIATED_TOKEN_PROGRAM_ID 
} from "@solana/spl-token";
import { assert } from "chai";

describe("nft_marketplace", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.NftMarketplace as Program<NftMarketplace>;

    const admin = Keypair.generate();
    const maker = Keypair.generate();
    const buyer = Keypair.generate();

    let nftMint: PublicKey;
    let paymentMint: PublicKey;

    let makerNftAta: PublicKey;
    let buyerNftAta: PublicKey;

    let makerPaymentAta: PublicKey;
    let buyerPaymentAta: PublicKey;
    let treasuryPaymentAta: PublicKey;

    const marketplaceName = "Turbin3Market";
    const feeBps = 200; // 2%
    const price = new anchor.BN(1000000); // 1 Token or 0.001 SOL
    const offerAmount = new anchor.BN(800000); // 0.0008 SOL

    // PDAs
    const [marketplacePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("marketplace"), Buffer.from(marketplaceName)],
        program.programId
    );

    const [treasuryPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("treasury"), marketplacePda.toBuffer()],
        program.programId
    );

    const [rewardsMintPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("rewards"), marketplacePda.toBuffer()],
        program.programId
    );

    let listingPda: PublicKey;
    let vaultPda: PublicKey;
    let offerPda: PublicKey;

    before(async () => {
        // Airdrop SOL
        const airdrops = [admin.publicKey, maker.publicKey, buyer.publicKey, treasuryPda].map(async (pubkey) => {
            const sig = await provider.connection.requestAirdrop(pubkey, 10 * anchor.web3.LAMPORTS_PER_SOL);
            const latestBlockhash = await provider.connection.getLatestBlockhash();
            await provider.connection.confirmTransaction({
                signature: sig,
                ...latestBlockhash
            });
        });
        await Promise.all(airdrops);

        // Create NFTs and Payment Mints
        nftMint = await createMint(provider.connection, maker, maker.publicKey, null, 0);
        paymentMint = await createMint(provider.connection, admin, admin.publicKey, null, 6);

        // Set up ATAs
        makerNftAta = await createAccount(provider.connection, maker, nftMint, maker.publicKey);
        buyerNftAta = await getAssociatedTokenAddress(nftMint, buyer.publicKey);

        makerPaymentAta = await getAssociatedTokenAddress(paymentMint, maker.publicKey);
        buyerPaymentAta = await createAccount(provider.connection, buyer, paymentMint, buyer.publicKey);
        treasuryPaymentAta = await getAssociatedTokenAddress(paymentMint, treasuryPda, true);

        // Mint NFT to Maker
        await mintTo(provider.connection, maker, nftMint, makerNftAta, maker, 1);

        // Mint Payment Tokens to Buyer
        await mintTo(provider.connection, admin, paymentMint, buyerPaymentAta, admin, 10000000); // 10 tokens

        // Find Listing and Vault PDAs
        [listingPda] = PublicKey.findProgramAddressSync(
            [marketplacePda.toBuffer(), maker.publicKey.toBuffer(), nftMint.toBuffer()],
            program.programId
        );

        vaultPda = await getAssociatedTokenAddress(nftMint, listingPda, true);

        // Find Offer PDA
        [offerPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("offer"), marketplacePda.toBuffer(), nftMint.toBuffer(), buyer.publicKey.toBuffer()],
            program.programId
        );
    });

    it("Is initialized!", async () => {
        const tx = await program.methods.initialize(marketplaceName, feeBps)
            .accounts({
                admin: admin.publicKey,
                marketplace: marketplacePda,
                treasury: treasuryPda,
                rewardsMint: rewardsMintPda,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .signers([admin])
            .rpc();
        
        const account = await program.account.marketplace.fetch(marketplacePda);
        assert.equal(account.fee, feeBps);
        assert.equal(account.name, marketplaceName);
    });

    it("Lists an NFT", async () => {
        const tx = await program.methods.list(price, null)
            .accounts({
                maker: maker.publicKey,
                marketplace: marketplacePda,
                makerMint: nftMint,
                makerAta: makerNftAta,
                listing: listingPda,
                vault: vaultPda,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .signers([maker])
            .rpc();
        
        const listingInfo = await program.account.listing.fetch(listingPda);
        assert.ok(listingInfo.price.eq(price));
        assert.equal(listingInfo.paymentMint, null);
    });

    it("Delists an NFT", async () => {
        const tx = await program.methods.delist()
            .accounts({
                maker: maker.publicKey,
                marketplace: marketplacePda,
                makerMint: nftMint,
                makerAta: makerNftAta,
                listing: listingPda,
                vault: vaultPda,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .signers([maker])
            .rpc();
        
        // Assert listing is closed
        const listingAccount = await provider.connection.getAccountInfo(listingPda);
        assert.isNull(listingAccount);
    });

    it("Lists an NFT for SPL Token", async () => {
        const tx = await program.methods.list(price, paymentMint)
            .accounts({
                maker: maker.publicKey,
                marketplace: marketplacePda,
                makerMint: nftMint,
                makerAta: makerNftAta,
                listing: listingPda,
                vault: vaultPda,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .signers([maker])
            .rpc();
        
        const listingInfo = await program.account.listing.fetch(listingPda);
        assert.equal(listingInfo.paymentMint.toString(), paymentMint.toString());
    });

    it("Buys an NFT with SPL Token", async () => {
        const tx = await program.methods.buyWithToken()
            .accounts({
                buyer: buyer.publicKey,
                maker: maker.publicKey,
                marketplace: marketplacePda,
                makerMint: nftMint,
                paymentMint: paymentMint,
                buyerPaymentAta: buyerPaymentAta,
                makerPaymentAta: makerPaymentAta,
                buyerNftAta: buyerNftAta,
                listing: listingPda,
                vault: vaultPda,
                treasury: treasuryPda,
                treasuryPaymentAta: treasuryPaymentAta,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .signers([buyer])
            .rpc();
        
        // Assert listing is closed
        const listingAccount = await provider.connection.getAccountInfo(listingPda);
        assert.isNull(listingAccount);
    });

    it("Lists an NFT again for offer testing", async () => {
        // Buyer currently holds the NFT in buyerNftAta, let's transfer it back to maker for simplicity, or just have buyer list it.
        // Actually, maker doesn't have the NFT anymore. Let's create a new NFT and list it.
        const newNftMint = await createMint(provider.connection, maker, maker.publicKey, null, 0);
        const newMakerNftAta = await createAccount(provider.connection, maker, newNftMint, maker.publicKey);
        await mintTo(provider.connection, maker, newNftMint, newMakerNftAta, maker, 1);

        nftMint = newNftMint;
        makerNftAta = newMakerNftAta;

        [listingPda] = PublicKey.findProgramAddressSync(
            [marketplacePda.toBuffer(), maker.publicKey.toBuffer(), nftMint.toBuffer()],
            program.programId
        );
        vaultPda = await getAssociatedTokenAddress(nftMint, listingPda, true);
        buyerNftAta = await getAssociatedTokenAddress(nftMint, buyer.publicKey);

        [offerPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("offer"), marketplacePda.toBuffer(), nftMint.toBuffer(), buyer.publicKey.toBuffer()],
            program.programId
        );

        await program.methods.list(price, null)
            .accounts({
                maker: maker.publicKey,
                marketplace: marketplacePda,
                makerMint: nftMint,
                makerAta: makerNftAta,
                listing: listingPda,
                vault: vaultPda,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .signers([maker])
            .rpc();
    });

    it("Makes an Offer", async () => {
        const tx = await program.methods.makeOffer(offerAmount)
            .accounts({
                buyer: buyer.publicKey,
                marketplace: marketplacePda,
                mint: nftMint,
                offer: offerPda,
                systemProgram: SystemProgram.programId,
            })
            .signers([buyer])
            .rpc();
        
        const offerInfo = await program.account.offer.fetch(offerPda);
        assert.ok(offerInfo.offerAmount.eq(offerAmount));
    });

    it("Accepts an Offer", async () => {
        const tx = await program.methods.acceptOffer()
            .accounts({
                maker: maker.publicKey,
                marketplace: marketplacePda,
                buyer: buyer.publicKey,
                makerMint: nftMint,
                buyerAta: buyerNftAta,
                listing: listingPda,
                vault: vaultPda,
                offer: offerPda,
                treasury: treasuryPda,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            })
            .signers([maker])
            .rpc();
        
        // Assert offer is closed
        const offerAccount = await provider.connection.getAccountInfo(offerPda);
        assert.isNull(offerAccount);
    });
});
